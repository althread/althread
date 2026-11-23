use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    hash::{Hash, Hasher},
};

use crate::ast::token::literal::Literal;

pub type ChannelsState = BTreeMap<(usize, String), Vec<Literal>>;

/// Key for a directed link that carries in-flight messages.
/// (from_pid, from_channel, to_pid, to_channel)
pub type ChannelLinkKey = (usize, String, usize, String);
pub type PendingDeliveriesState = BTreeMap<ChannelLinkKey, Vec<Literal>>;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Channels {
    /// states represent the input buffer of the channel
    /// that a process can read from.
    /// The key is a tuple of the program id and the channel name
    /// The value is a vector of literals
    /// the literals are tuples of the values that are sent
    states: ChannelsState,
    connections: BTreeMap<(usize, String), (usize, String)>,
    waiting_send: ChannelsState,

    /// Messages that have been sent but not yet delivered to the receiver mailbox.
    /// Keyed by (from_pid, from_channel, to_pid, to_channel).
    /// Delivery preserves per-link FIFO.
    pending_deliveries: PendingDeliveriesState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverInfo {
    pub program_id: usize,
    pub channel_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryInfo {
    pub from_program_id: usize,
    pub from_channel_name: String,
    pub to: ReceiverInfo,
    pub message: Literal,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            connections: BTreeMap::new(),
            waiting_send: BTreeMap::new(),
            pending_deliveries: BTreeMap::new(),
        }
    }

    /**
     * Send values to a channel. If the channel is not connected, the values are stored and the proc is waiting
     */
    pub fn send(
        &mut self,
        program_id: usize,
        channel_name: String,
        value: Literal,
        clock: usize,
    ) -> Option<ReceiverInfo> {
        let msg = Literal::Tuple(vec![
            Literal::Tuple(vec![
                Literal::Int(program_id as i64),
                Literal::Int(clock as i64),
            ]),
            value,
        ]);
        //^ added sender id to let the receiver know who sent a msg
        if let Some((to_program_id, to_channel_name)) =
            self.connections.get(&(program_id, channel_name.clone()))
        {
            let to_program_id = *to_program_id;
            let to_channel_name = to_channel_name.clone();
            let link_key: ChannelLinkKey = (
                program_id,
                channel_name,
                to_program_id,
                to_channel_name.clone(),
            );
            self.pending_deliveries
                .entry(link_key)
                .or_insert_with(Vec::new)
                .push(msg);

            return Some(ReceiverInfo {
                program_id: to_program_id,
                channel_name: to_channel_name,
            });
        }

        self.waiting_send
            .entry((program_id, channel_name))
            .or_insert(vec![])
            .push(msg);

        None
    }

    /**
     * Connect a proc to another proc
     * If the sender proc was waiting to send on the channel, it will send the values
     */
    pub fn connect(
        &mut self,
        program_id: usize,
        channel_name: String,
        to_program_id: usize,
        to_channel_name: String,
    ) -> Result<bool, String> {
        if self
            .connections
            .contains_key(&(program_id, channel_name.clone()))
        {
            return Err("This channel name is already used as a source on this process".into());
        }
        self.connections.insert(
            (program_id, channel_name.clone()),
            (to_program_id, to_channel_name.clone()),
        );

        if let Some(values) = self
            .waiting_send
            .remove(&(program_id, channel_name.clone()))
        {
            let link_key: ChannelLinkKey = (
                program_id,
                channel_name.clone(),
                to_program_id,
                to_channel_name.clone(),
            );
            self.pending_deliveries
                .entry(link_key)
                .or_insert_with(Vec::new)
                .extend(values);
            return Ok(true);
        }
        Ok(false)
    }

    /// Returns the list of links that currently have at least one pending message to deliver.
    pub fn pending_links(&self) -> Vec<ChannelLinkKey> {
        self.pending_deliveries
            .iter()
            .filter_map(|(k, v)| if v.is_empty() { None } else { Some(k.clone()) })
            .collect()
    }

    pub fn has_pending_deliveries(&self) -> bool {
        self.pending_deliveries.values().any(|v| !v.is_empty())
    }

    pub fn has_buffered_message(&self, program_id: usize, channel_name: &str) -> bool {
        self.states
            .get(&(program_id, channel_name.to_string()))
            .is_some_and(|queue| !queue.is_empty())
    }

    pub fn has_connection_from(&self, program_id: usize, channel_name: &str) -> bool {
        self.connections
            .contains_key(&(program_id, channel_name.to_string()))
    }

    /// Deliver exactly one pending message for a given link.
    pub fn deliver_one(&mut self, link: ChannelLinkKey) -> Option<DeliveryInfo> {
        let (from_pid, from_channel, to_pid, to_channel) = link.clone();
        let msg = {
            let queue = self.pending_deliveries.get_mut(&link)?;
            if queue.is_empty() {
                return None;
            }
            queue.remove(0)
        };
        self.states
            .entry((to_pid, to_channel.clone()))
            .or_insert_with(Vec::new)
            .push(msg.clone());

        // cleanup empty queues to keep state compact
        if let Some(queue) = self.pending_deliveries.get(&link) {
            if queue.is_empty() {
                self.pending_deliveries.remove(&link);
            }
        }

        Some(DeliveryInfo {
            from_program_id: from_pid,
            from_channel_name: from_channel,
            to: ReceiverInfo {
                program_id: to_pid,
                channel_name: to_channel,
            },
            message: msg,
        })
    }

    /**
     * Look at the values that are currently in the channel, return them if they exist without removing them
     */
    pub fn peek(&self, program_id: usize, channel_name: String) -> Option<&Literal> {
        match self.states.get(&(program_id, channel_name)) {
            Some(state) => {
                if let Some(Literal::Tuple(value)) = state.get(0) {
                    value.get(1) //extracts message content from entire message
                                 // ((prog_id, clock), content)
                } else {
                    None
                } //should be impossible to have something other than
                  //a tuple or nothing
            }
            None => None,
        }
    }

    /**
     * Pop the first values from the channel
     */
    pub fn pop(&mut self, program_id: usize, channel_name: String) -> Option<Literal> {
        match self.states.get_mut(&(program_id, channel_name)) {
            Some(state) => {
                let value = state.remove(0);
                if let Literal::Tuple(msg) = value {
                    msg.get(1).cloned()
                } else {
                    None
                } //should be impossible to have something other than
                  //a tuple or nothing
            }
            None => None,
        }
    }

    /// the state of the object is the state of each channel
    pub fn state(&self) -> &ChannelsState {
        &self.states
    }

    pub(super) fn same_execution_state(&self, other: &Self) -> bool {
        fn same_queues<K: Eq>(
            left: &BTreeMap<K, Vec<Literal>>,
            right: &BTreeMap<K, Vec<Literal>>,
        ) -> bool {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|((left_key, left), (right_key, right))| {
                        left_key == right_key
                            && left
                                .iter()
                                .map(message_payload)
                                .eq(right.iter().map(message_payload))
                    })
        }

        self.connections == other.connections
            && same_queues(&self.states, &other.states)
            && same_queues(&self.pending_deliveries, &other.pending_deliveries)
            && same_queues(&self.waiting_send, &other.waiting_send)
    }

    pub(super) fn hash_execution_state<H: Hasher>(&self, state: &mut H) {
        fn hash_queues<K: Hash, H: Hasher>(queues: &BTreeMap<K, Vec<Literal>>, state: &mut H) {
            queues.len().hash(state);
            for (key, messages) in queues {
                key.hash(state);
                messages.len().hash(state);
                for message in messages {
                    message_payload(message).hash(state);
                }
            }
        }

        self.connections.hash(state);
        hash_queues(&self.states, state);
        hash_queues(&self.pending_deliveries, state);
        hash_queues(&self.waiting_send, state);
    }

    pub(super) fn broadcast_targets(&self, pid: usize, pattern: &str) -> Vec<String> {
        self.connections
            .keys()
            .chain(self.waiting_send.keys())
            .filter(|(sender, name)| *sender == pid && name.starts_with(pattern))
            .map(|(_, name)| name)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn pending_deliveries(&self) -> &PendingDeliveriesState {
        &self.pending_deliveries
    }

    pub fn connections(&self) -> &BTreeMap<(usize, String), (usize, String)> {
        &self.connections
    }

    pub fn waiting_send(&self) -> &ChannelsState {
        &self.waiting_send
    }

    pub fn get_states(&self) -> ChannelsState {
        return self.states.clone();
    }

    pub fn get_pending_deliveries(&self) -> PendingDeliveriesState {
        self.pending_deliveries.clone()
    }

    pub fn get_connections(&self) -> HashMap<(usize, String), (usize, String)> {
        self.connections
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn get_waiting_send(&self) -> HashMap<(usize, String), Vec<Literal>> {
        self.waiting_send
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

// Only the payload is visible to instructions. The envelope identifies a
// particular send in an execution trace.
fn message_payload(message: &Literal) -> &Literal {
    let Literal::Tuple(fields) = message else {
        unreachable!("channel queues contain message envelopes");
    };
    &fields[1]
}

pub fn parse_message_tuple(msg: &Literal) -> Option<(usize, usize, String)> {
    let (sender_id, sender_clock) = message_trace_id(msg)?;
    Some((sender_id, sender_clock, message_payload(msg).to_string()))
}

pub(super) fn message_trace_id(msg: &Literal) -> Option<(usize, usize)> {
    // Expected format: ((sender_id, sender_clock), content)
    let Literal::Tuple(msg_tuple) = msg else {
        return None;
    };
    if msg_tuple.len() < 2 {
        return None;
    }
    let Literal::Tuple(sender_info) = msg_tuple.get(0)? else {
        return None;
    };
    if sender_info.len() < 2 {
        return None;
    }
    let Literal::Int(sender_id) = sender_info.get(0)? else {
        return None;
    };
    let Literal::Int(sender_clock) = sender_info.get(1)? else {
        return None;
    };

    Some((*sender_id as usize, *sender_clock as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_channel_identity_ignores_trace_ids_but_keeps_fifo_payloads() {
        use std::collections::hash_map::DefaultHasher;

        let mut left = Channels::new();
        let mut right = Channels::new();
        left.send(0, "out".into(), Literal::Int(1), 1);
        left.send(0, "out".into(), Literal::Int(2), 2);
        left.send(0, "other".into(), Literal::Int(3), 3);

        right.send(0, "other".into(), Literal::Int(3), 4);
        right.send(0, "out".into(), Literal::Int(1), 5);
        right.send(0, "out".into(), Literal::Int(2), 6);

        let hash = |channels: &Channels| {
            let mut state = DefaultHasher::new();
            channels.hash_execution_state(&mut state);
            state.finish()
        };
        assert_ne!(left, right);
        assert!(left.same_execution_state(&right));
        assert_eq!(hash(&left), hash(&right));

        right
            .waiting_send
            .get_mut(&(0, "out".into()))
            .unwrap()
            .swap(0, 1);
        assert!(!left.same_execution_state(&right));
    }

    #[test]
    fn send_is_not_immediately_visible_until_delivery() {
        let mut channels = Channels::new();

        channels
            .connect(1, "out".to_string(), 0, "in".to_string())
            .unwrap();

        let receiver = channels.send(1, "out".to_string(), Literal::Int(42), 1);
        assert_eq!(receiver.unwrap().program_id, 0);

        // Not delivered yet
        assert_eq!(channels.peek(0, "in".to_string()), None);
        assert!(channels.has_pending_deliveries());

        channels
            .deliver_one((1, "out".to_string(), 0, "in".to_string()))
            .unwrap();
        assert_eq!(channels.peek(0, "in".to_string()), Some(&Literal::Int(42)));
    }

    #[test]
    fn delivery_can_interleave_between_senders() {
        let mut channels = Channels::new();

        channels
            .connect(1, "out".to_string(), 0, "in".to_string())
            .unwrap();
        channels
            .connect(2, "out".to_string(), 0, "in".to_string())
            .unwrap();

        channels.send(1, "out".to_string(), Literal::Int(1), 1);
        channels.send(2, "out".to_string(), Literal::Int(2), 1);

        // Choose to deliver sender 2 first, then sender 1.
        channels
            .deliver_one((2, "out".to_string(), 0, "in".to_string()))
            .unwrap();
        channels
            .deliver_one((1, "out".to_string(), 0, "in".to_string()))
            .unwrap();

        assert_eq!(channels.pop(0, "in".to_string()), Some(Literal::Int(2)));
        assert_eq!(channels.pop(0, "in".to_string()), Some(Literal::Int(1)));
    }
}

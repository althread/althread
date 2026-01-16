use std::collections::{BTreeMap, HashMap};

use crate::ast::token::literal::Literal;

pub type ChannelsState = BTreeMap<(usize, String), Vec<Literal>>;

/// Key for a directed link that carries in-flight messages.
/// (from_pid, from_channel, to_pid, to_channel)
pub type ChannelLinkKey = (usize, String, usize, String);
pub type PendingDeliveriesState = BTreeMap<ChannelLinkKey, Vec<Literal>>;

#[derive(Debug, PartialEq, Clone)]
pub struct Channels {
    /// states represent the input buffer of the channel
    /// that a process can read from.
    /// The key is a tuple of the program id and the channel name
    /// The value is a vector of literals
    /// the literals are tuples of the values that are sent
    states: ChannelsState,
    connections: HashMap<(usize, String), (usize, String)>,
    waiting_send: HashMap<(usize, String), Vec<Literal>>,

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
}

impl Channels {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            connections: HashMap::new(),
            waiting_send: HashMap::new(),
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
            let link_key: ChannelLinkKey = (
                program_id,
                channel_name.clone(),
                *to_program_id,
                to_channel_name.clone(),
            );
            self.pending_deliveries
                .entry(link_key)
                .or_insert_with(Vec::new)
                .push(msg);

            return Some(ReceiverInfo {
                program_id: *to_program_id,
                channel_name: to_channel_name.clone(),
            });
        }

        self.waiting_send
            .entry((program_id, channel_name.clone()))
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
            .push(msg);

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

    pub fn get_states(&self) -> ChannelsState {
        return self.states.clone();
    }

    pub fn get_pending_deliveries(&self) -> PendingDeliveriesState {
        self.pending_deliveries.clone()
    }

    pub fn get_connections(&self) -> HashMap<(usize, String), (usize, String)> {
        self.connections.clone()
    }

    pub fn get_waiting_send(&self) -> HashMap<(usize, String), Vec<Literal>> {
        self.waiting_send.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

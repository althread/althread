use std::collections::{BTreeMap, HashMap};

use crate::ast::token::literal::Literal;

pub type ChannelsState = BTreeMap<(usize, String), Vec<Literal>>;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverInfo {
    pub program_id: usize,
    pub channel_name: String,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            connections: HashMap::new(),
            waiting_send: HashMap::new(),
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
    ) -> Option<ReceiverInfo> {
        if let Some((to_program_id, to_channel_name)) =
            self.connections.get(&(program_id, channel_name.clone()))
        {
            // get the state of the channel (create it if it doesn't exist)
            if let Some(state) = self
                .states
                .get_mut(&(*to_program_id, to_channel_name.clone()))
            {
                state.push(value);
            } else {
                self.states
                    .insert((*to_program_id, to_channel_name.clone()), vec![value]);
            }
            return Some(ReceiverInfo {
                program_id: *to_program_id,
                channel_name: to_channel_name.clone(),
            });
        }

        self.waiting_send
            .entry((program_id, channel_name.clone()))
            .or_insert(vec![])
            .push(value);

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
    ) -> Result<Option<(usize, String)>, String> {
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
            self.states
                .entry((to_program_id, to_channel_name.clone()))
                .or_insert(vec![])
                .extend(values);
            return Ok(Some((program_id, channel_name)));
        }
        Ok(None)
    }

    /**
     * Look at the values that are currently in the channel, return them if they exist without removing them
     */
    pub fn peek(&self, program_id: usize, channel_name: String) -> Option<&Literal> {
        match self.states.get(&(program_id, channel_name)) {
            Some(state) => state.get(0),
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
                Some(value)
            }
            None => None,
        }
    }

    /// the state of the object is the state of each channel
    pub fn state(&self) -> &ChannelsState {
        &self.states
    }
}

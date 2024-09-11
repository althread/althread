use std::collections::HashMap;

use crate::ast::token::literal::Literal;



pub struct Channels {
    states: HashMap<(usize, String), Vec<Vec<Literal>>>,
    connections: HashMap<(usize, String), (usize, String)>,
    waiting_proc: HashMap<usize, (String, Vec<Literal>)>,
}

pub struct ReceiverInfo {
    pub program_id: usize,
    pub channel_name: String,
}

impl Channels {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            connections: HashMap::new(),
            waiting_proc: HashMap::new(),
        }
    }

    /**
     * Send values to a channel. If the channel is not connected, the values are stored and the proc is waiting
     */
    pub fn send(&mut self, program_id: usize, channel_name: String, values: Vec<Literal>) -> Option<ReceiverInfo> {
        if let Some((to_program_id, to_channel_name)) = self.connections.get(&(program_id, channel_name.clone())) {
            // get the state of the channel (create it if it doesn't exist)
            if let Some(state) = self.states.get_mut(&(*to_program_id, to_channel_name.clone())) {
                state.push(values);
            } else {
                self.states.insert((*to_program_id, to_channel_name.clone()), vec![values]);
            }
            return Some(ReceiverInfo {
                program_id: *to_program_id,
                channel_name: to_channel_name.clone(),
            });
        }

        assert!(self.waiting_proc.get(&program_id).is_none(), "A proc can only wait on one channel");
        self.waiting_proc.insert(program_id, (channel_name.clone(), values));
        None

    }

    /**
     * Check if the proc is currently waiting
     */
    pub fn is_waiting(&self, program_id: usize) -> bool {
        self.waiting_proc.contains_key(&program_id)
    }

    /**
     * Connect a proc to another proc
     * If the sender proc was waiting to send on the channel, it will send the values
     */
    pub fn connect(&mut self, program_id: usize, channel_name: String, to_program_id: usize, to_channel_name: String) -> Option<(usize, String)> {
        self.connections.insert((program_id, channel_name.clone()), (to_program_id, to_channel_name.clone()));

        if let Some((channel_waiting, values)) = self.waiting_proc.get(&program_id) {
            if channel_waiting == &channel_name {
                let (_, values) = self.waiting_proc.remove(&program_id).unwrap();
                self.send(program_id, channel_name.clone(), values);
                return Some((program_id, channel_name));
            }
        }
        None
    }

    /**
     * Look at the values that are currently in the channel, return them if they exist without removing them
     */
    pub fn peek(&self, program_id: usize, channel_name: String) -> Option<&Vec<Literal>> {
        match self.states.get(&(program_id, channel_name)) {
            Some(state) => state.get(0),
            None => None,
        }
    }

    /**
     * Pop the first values from the channel
     */
    pub fn pop(&mut self, program_id: usize, channel_name: String) -> Option<Vec<Literal>> {
        match self.states.get_mut(&(program_id, channel_name)) {
            Some(state) => {
                let values = state.remove(0);
                Some(values)
            },
            None => None,
        }
    }
}
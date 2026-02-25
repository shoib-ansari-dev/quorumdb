use std::cmp::min;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command{
    Set{ key: String, value: String },
    Delete{ key: String },
    Noop
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry{
    pub term: u64,
    pub index: u64,
    pub command: Command
}


impl LogEntry{
    pub fn new(term: u64, index: u64, command: Command) -> Self{
        Self{
            term,
            index,
            command
        }
    }
    pub fn noop(term: u64, index: u64) -> Self{
        Self{
            term,
            index,
            command: Command::Noop
        }
    }
}

#[derive(Debug, Clone)]
pub struct RaftLog{
    entries: Vec<LogEntry>,
    last_index: u64,
    last_term: u64
}

impl Default for RaftLog{
    fn default() -> Self {
        Self::new()
    }
}

impl RaftLog {
    pub fn new() -> Self {
        Self{
            entries: Vec::new(),
            last_index: 0,
            last_term: 0
        }
    }

    pub fn append(&mut self, term: u64, command: Command) -> u64 {
        let index = self.last_index + 1;
        let entry = LogEntry::new(term, index, command);
        self.entries.push(entry);
        self.last_index = index;
        self.last_term = term;
        index
    }

    pub fn append_multiple_entries(&mut self, entries: Vec<LogEntry>){
        for entry in entries {
            if entry.index > self.last_index{
                self.last_index = entry.index;
                self.last_term = entry.term;
                self.entries.push(entry);
            }
        }
    }

    pub fn get(&self, index: u64) -> Option<&LogEntry>{
        if index == 0 || index> self.last_index{
            return None;
        }
        self.entries.get((index - 1) as usize)
    }

    pub fn term_at(&self, index: u64) -> Option<u64>{
        if index == 0 {
            return Some(0);
        }
        if index > self.last_index {
            return None;
        }
        self.get(index).map(|e| e.term)
    }

    pub fn get_range(&self, start_index: u64, end_index: u64) -> Vec<LogEntry>{
        if start_index == 0 || start_index > self.last_index{
            return Vec::new();
        }
        let start = (start_index - 1) as usize;
        let end = min(end_index as usize, self.entries.len());

        self.entries[start..end].to_vec()
    }

    pub fn get_from(&self, start_index: u64) -> Vec<LogEntry>{
        if start_index == 0 || start_index > self.last_index {
            return Vec::new();
        }
        let start = (start_index - 1) as usize;
        self.entries[start..].to_vec()
    }

    pub fn last_index(&self) -> u64{
        self.last_index
    }

    pub fn last_term(&self) -> u64{
        self.last_term
    }

    pub fn is_up_to_date(&self, candidate_last_term: u64, candidate_last_index: u64) -> bool{
        if candidate_last_term != self.last_term{
            return candidate_last_term >= self.last_term;
        }
        candidate_last_index >= self.last_index
    }

    pub fn matches(&self, prev_log_index: u64, prev_log_term: u64) -> bool{
        if prev_log_index == 0 {
            return true;
        }
        match self.term_at(prev_log_index) {
            Some(term) => term == prev_log_term,
            None => false,
        }
    }

    pub fn truncate_after(&mut self, index: u64){
        if index >= self.last_index {
            return;
        }

        self.entries.truncate(index as usize);

        if index == 0 {
            self.last_index = 0;
            self.last_term = 0;
        } else {
            self.last_index = index;
            self.last_term = self.entries.last().map(|e| e.term).unwrap_or(0);
        }
    }

    pub fn handle_append_entries(
        &mut self,
        prev_log_index: u64,
        prev_log_term: u64,
        entry: Vec<LogEntry>,
    ) -> bool{
        if !self.matches(prev_log_index, prev_log_term) {
            return false;
        }
        for entry in entry {
            if let Some(existing) = self.get(entry.index) {
                if existing.term != entry.term {
                    self.truncate_after(entry.index - 1);
                    self.entries.push(entry.clone());
                    self.last_index = entry.index;
                    self.last_term = entry.term;
                }
            } else {
                self.entries.push(entry.clone());
                self.last_index = entry.index;
                self.last_term = entry.term;
            }
        }
        true
    }

    pub fn len(&self) -> usize{
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool{
        self.entries.is_empty()
    }
}

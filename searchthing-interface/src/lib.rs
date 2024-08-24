use std::ops::Range;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SearchItemHandle(pub i32);

impl SearchItemHandle {
    /// terminates match list for simplicity in c compatible interface
    pub const TERMINATOR: Self = Self(-1);
}

pub struct SearcherInfo<'a> {
    pub name: &'a str,
    pub icon: &'a str,
}

pub struct MatchInfo<'a> {
    pub name: &'a str,
    pub desc: &'a str,
    pub icon: &'a str,
}

/// required methods for a SearchThing module
pub trait SearchModule {
    fn info(&self) -> SearcherInfo;

    fn queery(&self, input: &str, max_returned: u32) -> Vec<SearchItemHandle>;

    fn get_match_info(&self, item: SearchItemHandle) -> MatchInfo;

    // NOTE: the word handle is used with two different meanings here
    fn handle_selection(&self, selection: SearchItemHandle);
}

pub fn substring_range(string: &str, substring: &str) -> Option<Range<usize>> {
    let start = string.find(&substring.to_lowercase());
    start.map(|si| si..si + substring.len())
}

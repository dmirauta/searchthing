use std::ops::Range;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

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
    fn mod_info(&self) -> SearcherInfo;

    fn queery(&self, input: &str, max_returned: u32) -> Vec<SearchItemHandle>;

    fn match_info(&self, item: SearchItemHandle) -> MatchInfo;

    // NOTE: the word handle is used with two different meanings here
    fn handle_selection(&self, selection: SearchItemHandle);
}

/// second argument is changed to lowercase within the function, first is not
pub fn substring_range(string: &str, substring: &str) -> Option<Range<usize>> {
    let start = string.find(&substring.to_lowercase());
    start.map(|si| si..si + substring.len())
}

// TODO: Merge [SearchMethod::match_idxs] output into [MatchInfo], so as to not have to compute
// the match twice or assume how a possibly external searcher is searching

pub trait SearchMethod {
    fn match_idxs(search_text: &str, queery_text: &str) -> Option<(i64, Vec<usize>)>;
}

#[allow(dead_code)]
pub struct BasicSearch;

impl SearchMethod for BasicSearch {
    fn match_idxs(search_text: &str, queery_text: &str) -> Option<(i64, Vec<usize>)> {
        substring_range(search_text, queery_text).map(|r| (-(r.start as i64), r.collect()))
    }
}

pub struct FuzzySearch;

impl SearchMethod for FuzzySearch {
    fn match_idxs(search_text: &str, queery_text: &str) -> Option<(i64, Vec<usize>)> {
        SkimMatcherV2::default().fuzzy_indices(search_text, queery_text)
    }
}

pub fn char_from_codepoint(codepoint: &str) -> Option<char> {
    let u = u32::from_str_radix(&codepoint[2..], 16).ok()?;
    char::from_u32(u)
}

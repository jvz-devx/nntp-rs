//! RFC 3977 - Network News Transfer Protocol (NNTP)
//!
//! These tests verify compliance with the core NNTP protocol specification.
//! https://datatracker.ietf.org/doc/html/rfc3977

mod rfc3977 {
    mod capabilities;
    mod commands;
    mod errors;
    mod group;
    mod hdr;
    mod help;
    mod ihave;
    mod list;
    mod listgroup;
    mod mode;
    mod multiline;
    mod navigation;
    mod newgroups;
    mod newnews;
    mod over;
    mod parsing;
    mod post;
    mod response;
    mod stat;
    mod xover;
}

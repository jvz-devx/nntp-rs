//! RFC 5537 Section 5 - Control Message Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc5537#section-5

use nntp_rs::article::{Article, ControlMessage, Headers};
#[test]
fn test_is_control_message_true() {
    let mut headers = Headers::new(
        "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
        "admin@example.com".to_string(),
        "<cancel123@example.com>".to_string(),
        vec!["comp.lang.rust".to_string()],
        "news.example.com!not-for-mail".to_string(),
        "cancel message".to_string(),
    );
    headers.control = Some("cancel <spam@example.com>".to_string());

    let article = Article::new(headers, String::new());
    assert!(article.is_control_message());
}

#[test]
fn test_is_control_message_false() {
    let headers = Headers::new(
        "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
        "user@example.com".to_string(),
        "<article123@example.com>".to_string(),
        vec!["comp.lang.rust".to_string()],
        "news.example.com!not-for-mail".to_string(),
        "Regular Article".to_string(),
    );

    let article = Article::new(headers, "This is a normal article.".to_string());
    assert!(!article.is_control_message());
}
#[test]
fn test_parse_cancel_basic() {
    let msg = ControlMessage::parse("cancel <spam@example.com>").unwrap();
    match msg {
        ControlMessage::Cancel { message_id } => {
            assert_eq!(message_id, "<spam@example.com>");
        }
        _ => panic!("Expected Cancel control message"),
    }
}

#[test]
fn test_parse_cancel_uppercase() {
    let msg = ControlMessage::parse("CANCEL <spam@example.com>").unwrap();
    match msg {
        ControlMessage::Cancel { message_id } => {
            assert_eq!(message_id, "<spam@example.com>");
        }
        _ => panic!("Expected Cancel control message"),
    }
}

#[test]
fn test_parse_cancel_extra_whitespace() {
    let msg = ControlMessage::parse("  cancel   <spam@example.com>  ").unwrap();
    match msg {
        ControlMessage::Cancel { message_id } => {
            assert_eq!(message_id, "<spam@example.com>");
        }
        _ => panic!("Expected Cancel control message"),
    }
}

#[test]
fn test_parse_cancel_complex_message_id() {
    let msg = ControlMessage::parse("cancel <part1of10.abc123.xyz@news.example.com>").unwrap();
    match msg {
        ControlMessage::Cancel { message_id } => {
            assert_eq!(message_id, "<part1of10.abc123.xyz@news.example.com>");
        }
        _ => panic!("Expected Cancel control message"),
    }
}

#[test]
fn test_parse_cancel_missing_message_id() {
    let msg = ControlMessage::parse("cancel").unwrap();
    match msg {
        ControlMessage::Unknown { .. } => {
            // Malformed cancel becomes Unknown
        }
        _ => panic!("Expected Unknown control message for malformed cancel"),
    }
}

#[test]
fn test_article_parse_control_cancel() {
    let mut headers = Headers::new(
        "Mon, 20 Jan 2025 12:00:00 +0000".to_string(),
        "admin@example.com".to_string(),
        "<cancel123@example.com>".to_string(),
        vec!["comp.lang.rust".to_string()],
        "news.example.com!not-for-mail".to_string(),
        "cancel spam message".to_string(),
    );
    headers.control = Some("cancel <spam123@example.com>".to_string());

    let article = Article::new(headers, String::new());
    let control = article.parse_control_message().unwrap();

    match control {
        ControlMessage::Cancel { message_id } => {
            assert_eq!(message_id, "<spam123@example.com>");
        }
        _ => panic!("Expected Cancel"),
    }
}
#[test]
fn test_parse_newgroup_basic() {
    let msg = ControlMessage::parse("newgroup comp.lang.rust").unwrap();
    match msg {
        ControlMessage::Newgroup { group, moderated } => {
            assert_eq!(group, "comp.lang.rust");
            assert!(!moderated);
        }
        _ => panic!("Expected Newgroup control message"),
    }
}

#[test]
fn test_parse_newgroup_moderated() {
    let msg = ControlMessage::parse("newgroup comp.lang.rust moderated").unwrap();
    match msg {
        ControlMessage::Newgroup { group, moderated } => {
            assert_eq!(group, "comp.lang.rust");
            assert!(moderated);
        }
        _ => panic!("Expected Newgroup control message"),
    }
}

#[test]
fn test_parse_newgroup_moderated_uppercase() {
    let msg = ControlMessage::parse("newgroup comp.lang.rust MODERATED").unwrap();
    match msg {
        ControlMessage::Newgroup { group, moderated } => {
            assert_eq!(group, "comp.lang.rust");
            assert!(moderated);
        }
        _ => panic!("Expected Newgroup control message"),
    }
}

#[test]
fn test_parse_newgroup_missing_group() {
    let msg = ControlMessage::parse("newgroup").unwrap();
    match msg {
        ControlMessage::Unknown { .. } => {
            // Malformed newgroup becomes Unknown
        }
        _ => panic!("Expected Unknown control message for malformed newgroup"),
    }
}

#[test]
fn test_parse_newgroup_hierarchy() {
    let msg = ControlMessage::parse("newgroup alt.binaries.test.moderated moderated").unwrap();
    match msg {
        ControlMessage::Newgroup { group, moderated } => {
            assert_eq!(group, "alt.binaries.test.moderated");
            assert!(moderated);
        }
        _ => panic!("Expected Newgroup control message"),
    }
}
#[test]
fn test_parse_rmgroup_basic() {
    let msg = ControlMessage::parse("rmgroup alt.test").unwrap();
    match msg {
        ControlMessage::Rmgroup { group } => {
            assert_eq!(group, "alt.test");
        }
        _ => panic!("Expected Rmgroup control message"),
    }
}

#[test]
fn test_parse_rmgroup_uppercase() {
    let msg = ControlMessage::parse("RMGROUP alt.test").unwrap();
    match msg {
        ControlMessage::Rmgroup { group } => {
            assert_eq!(group, "alt.test");
        }
        _ => panic!("Expected Rmgroup control message"),
    }
}

#[test]
fn test_parse_rmgroup_missing_group() {
    let msg = ControlMessage::parse("rmgroup").unwrap();
    match msg {
        ControlMessage::Unknown { .. } => {
            // Malformed rmgroup becomes Unknown
        }
        _ => panic!("Expected Unknown control message for malformed rmgroup"),
    }
}
#[test]
fn test_parse_checkgroups_no_args() {
    let msg = ControlMessage::parse("checkgroups").unwrap();
    match msg {
        ControlMessage::Checkgroups { scope, serial } => {
            assert!(scope.is_none());
            assert!(serial.is_none());
        }
        _ => panic!("Expected Checkgroups control message"),
    }
}

#[test]
fn test_parse_checkgroups_with_scope() {
    let msg = ControlMessage::parse("checkgroups comp").unwrap();
    match msg {
        ControlMessage::Checkgroups { scope, serial } => {
            assert_eq!(scope.unwrap(), "comp");
            assert!(serial.is_none());
        }
        _ => panic!("Expected Checkgroups control message"),
    }
}

#[test]
fn test_parse_checkgroups_with_serial() {
    let msg = ControlMessage::parse("checkgroups #1234").unwrap();
    match msg {
        ControlMessage::Checkgroups { scope, serial } => {
            assert!(scope.is_none());
            assert_eq!(serial.unwrap(), "#1234");
        }
        _ => panic!("Expected Checkgroups control message"),
    }
}

#[test]
fn test_parse_checkgroups_with_scope_and_serial() {
    let msg = ControlMessage::parse("checkgroups comp #5678").unwrap();
    match msg {
        ControlMessage::Checkgroups { scope, serial } => {
            assert_eq!(scope.unwrap(), "comp");
            assert_eq!(serial.unwrap(), "#5678");
        }
        _ => panic!("Expected Checkgroups control message"),
    }
}
#[test]
fn test_parse_ihave_single_message_id() {
    let msg = ControlMessage::parse("ihave <msg1@example.com>").unwrap();
    match msg {
        ControlMessage::Ihave {
            message_ids,
            relayer,
        } => {
            assert_eq!(message_ids.len(), 1);
            assert_eq!(message_ids[0], "<msg1@example.com>");
            assert!(relayer.is_none());
        }
        _ => panic!("Expected Ihave control message"),
    }
}

#[test]
fn test_parse_ihave_multiple_message_ids() {
    let msg = ControlMessage::parse("ihave <msg1@example.com> <msg2@example.com>").unwrap();
    match msg {
        ControlMessage::Ihave {
            message_ids,
            relayer,
        } => {
            assert_eq!(message_ids.len(), 2);
            assert_eq!(message_ids[0], "<msg1@example.com>");
            assert_eq!(message_ids[1], "<msg2@example.com>");
            assert!(relayer.is_none());
        }
        _ => panic!("Expected Ihave control message"),
    }
}

#[test]
fn test_parse_ihave_with_relayer() {
    let msg = ControlMessage::parse("ihave <msg1@example.com> news.server.com").unwrap();
    match msg {
        ControlMessage::Ihave {
            message_ids,
            relayer,
        } => {
            assert_eq!(message_ids.len(), 1);
            assert_eq!(message_ids[0], "<msg1@example.com>");
            assert_eq!(relayer.unwrap(), "news.server.com");
        }
        _ => panic!("Expected Ihave control message"),
    }
}

#[test]
fn test_parse_ihave_missing_message_id() {
    let msg = ControlMessage::parse("ihave").unwrap();
    match msg {
        ControlMessage::Unknown { .. } => {
            // Malformed ihave becomes Unknown
        }
        _ => panic!("Expected Unknown control message for malformed ihave"),
    }
}
#[test]
fn test_parse_sendme_single_message_id() {
    let msg = ControlMessage::parse("sendme <msg1@example.com>").unwrap();
    match msg {
        ControlMessage::Sendme {
            message_ids,
            relayer,
        } => {
            assert_eq!(message_ids.len(), 1);
            assert_eq!(message_ids[0], "<msg1@example.com>");
            assert!(relayer.is_none());
        }
        _ => panic!("Expected Sendme control message"),
    }
}

#[test]
fn test_parse_sendme_with_relayer() {
    let msg = ControlMessage::parse("sendme <msg1@example.com> news.peer.com").unwrap();
    match msg {
        ControlMessage::Sendme {
            message_ids,
            relayer,
        } => {
            assert_eq!(message_ids.len(), 1);
            assert_eq!(message_ids[0], "<msg1@example.com>");
            assert_eq!(relayer.unwrap(), "news.peer.com");
        }
        _ => panic!("Expected Sendme control message"),
    }
}
#[test]
fn test_parse_unknown_control_message() {
    let msg = ControlMessage::parse("customcommand arg1 arg2").unwrap();
    match msg {
        ControlMessage::Unknown { value } => {
            assert_eq!(value, "customcommand arg1 arg2");
        }
        _ => panic!("Expected Unknown control message"),
    }
}

#[test]
fn test_parse_obsolete_sendsys() {
    // Obsolete control message (RFC 5537 Section 5.6)
    let msg = ControlMessage::parse("sendsys").unwrap();
    match msg {
        ControlMessage::Unknown { value } => {
            assert_eq!(value, "sendsys");
        }
        _ => panic!("Expected Unknown control message for obsolete sendsys"),
    }
}

#[test]
fn test_parse_empty_control() {
    let msg = ControlMessage::parse("");
    assert!(msg.is_none());
}

#[test]
fn test_parse_whitespace_only_control() {
    let msg = ControlMessage::parse("   ");
    assert!(msg.is_none());
}

// Real-World Control Message Examples

#[test]
fn test_real_world_cancel_with_path() {
    let mut headers = Headers::new(
        "Mon, 20 Jan 2025 12:34:56 +0000".to_string(),
        "admin@news.example.com".to_string(),
        "<cancel.xyz789@news.example.com>".to_string(),
        vec!["comp.lang.rust".to_string()],
        "news.example.com!feed.example.org!not-for-mail".to_string(),
        "cancel spam post".to_string(),
    );
    headers.control = Some("cancel <spam.abc123@spammer.com>".to_string());
    headers.approved = Some("admin@news.example.com".to_string());

    let article = Article::new(headers, "This article violates our policies.".to_string());

    assert!(article.is_control_message());
    let control = article.parse_control_message().unwrap();

    match control {
        ControlMessage::Cancel { message_id } => {
            assert_eq!(message_id, "<spam.abc123@spammer.com>");
        }
        _ => panic!("Expected Cancel"),
    }
}

#[test]
fn test_real_world_newgroup_moderated() {
    let mut headers = Headers::new(
        "Mon, 20 Jan 2025 10:00:00 +0000".to_string(),
        "newsadmin@isc.org".to_string(),
        "<newgroup.comp.lang.rust@control.isc.org>".to_string(),
        vec!["comp.lang.rust".to_string()],
        "control.isc.org!not-for-mail".to_string(),
        "newgroup comp.lang.rust moderated".to_string(),
    );
    headers.control = Some("newgroup comp.lang.rust moderated".to_string());
    headers.approved = Some("newsadmin@isc.org".to_string());

    let article = Article::new(
        headers,
        "For discussion of the Rust programming language.".to_string(),
    );

    assert!(article.is_control_message());
    assert!(article.headers.approved.is_some());

    let control = article.parse_control_message().unwrap();
    match control {
        ControlMessage::Newgroup { group, moderated } => {
            assert_eq!(group, "comp.lang.rust");
            assert!(moderated);
        }
        _ => panic!("Expected Newgroup"),
    }
}

#[test]
fn test_real_world_checkgroups_with_body() {
    let mut headers = Headers::new(
        "Mon, 20 Jan 2025 08:00:00 +0000".to_string(),
        "group-admin@news.admin.net".to_string(),
        "<checkgroups.comp@control.news.admin.net>".to_string(),
        vec!["comp.admin.policy".to_string()],
        "control.news.admin.net!not-for-mail".to_string(),
        "checkgroups comp hierarchy".to_string(),
    );
    headers.control = Some("checkgroups comp #12345".to_string());
    headers.approved = Some("group-admin@news.admin.net".to_string());

    let body = "comp.lang.c\ncomp.lang.c++\ncomp.lang.rust\n".to_string();
    let article = Article::new(headers, body);

    assert!(article.is_control_message());
    let control = article.parse_control_message().unwrap();

    match control {
        ControlMessage::Checkgroups { scope, serial } => {
            assert_eq!(scope.unwrap(), "comp");
            assert_eq!(serial.unwrap(), "#12345");
        }
        _ => panic!("Expected Checkgroups"),
    }

    // Body should contain the group list
    assert!(article.body.contains("comp.lang.rust"));
}
#[test]
fn test_control_message_equality() {
    let msg1 = ControlMessage::Cancel {
        message_id: "<spam@example.com>".to_string(),
    };
    let msg2 = ControlMessage::Cancel {
        message_id: "<spam@example.com>".to_string(),
    };
    assert_eq!(msg1, msg2);
}

#[test]
fn test_control_message_inequality() {
    let msg1 = ControlMessage::Cancel {
        message_id: "<spam1@example.com>".to_string(),
    };
    let msg2 = ControlMessage::Cancel {
        message_id: "<spam2@example.com>".to_string(),
    };
    assert_ne!(msg1, msg2);
}

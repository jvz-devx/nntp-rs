//! RFC 6048 Section 4 - LIST DISTRIBUTIONS Tests
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6048#section-4

use nntp_rs::{NntpResponse, codes, commands};
#[test]
fn test_list_distributions_command_format() {
    let cmd = commands::list_distributions();
    assert_eq!(cmd, "LIST DISTRIBUTIONS\r\n");
}

#[test]
fn test_list_distributions_ends_with_crlf() {
    let cmd = commands::list_distributions();
    assert!(cmd.ends_with("\r\n"));
    assert_eq!(cmd.matches("\r\n").count(), 1);
}

#[test]
fn test_list_distributions_command_uppercase() {
    let cmd = commands::list_distributions();
    assert!(cmd.starts_with("LIST DISTRIBUTIONS"));
}

#[test]
fn test_list_distributions_no_arguments() {
    let cmd = commands::list_distributions();
    // Should be exactly "LIST DISTRIBUTIONS\r\n" with no arguments
    assert_eq!(cmd, "LIST DISTRIBUTIONS\r\n");
}
#[test]
fn test_parse_list_distributions_response_basic() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List of distributions follows".to_string(),
        lines: vec![
            "local Local to this news server.".to_string(),
            "usa Local to the United States of America.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 2);

    assert_eq!(distributions[0].name, "local");
    assert_eq!(distributions[0].description, "Local to this news server.");

    assert_eq!(distributions[1].name, "usa");
    assert_eq!(
        distributions[1].description,
        "Local to the United States of America."
    );
}

#[test]
fn test_parse_list_distributions_response_empty() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 0);
}

#[test]
fn test_parse_list_distributions_response_single() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec!["thissite Local to this site.".to_string()],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 1);
    assert_eq!(distributions[0].name, "thissite");
    assert_eq!(distributions[0].description, "Local to this site.");
}

#[test]
fn test_parse_list_distributions_response_multiple() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Distributions list follows".to_string(),
        lines: vec![
            "fr Local to France.".to_string(),
            "local Local to this news server.".to_string(),
            "thissite Local to this site.".to_string(),
            "usa Local to the United States of America.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 4);
    assert_eq!(distributions[0].name, "fr");
    assert_eq!(distributions[1].name, "local");
    assert_eq!(distributions[2].name, "thissite");
    assert_eq!(distributions[3].name, "usa");
}

#[test]
fn test_parse_list_distributions_response_with_periods() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "local Local distribution.".to_string(),
            "usa U.S.A. distribution area.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 2);
    assert_eq!(distributions[0].description, "Local distribution.");
    assert_eq!(distributions[1].description, "U.S.A. distribution area.");
}
#[test]
fn test_parse_list_distributions_response_protocol_error() {
    let response = NntpResponse {
        code: 500,
        message: "Command not recognized".to_string(),
        lines: vec![],
    };

    let result = commands::parse_list_distributions_response(response);
    assert!(result.is_err());
}

#[test]
fn test_parse_list_distributions_response_wrong_code() {
    let response = NntpResponse {
        code: 200, // Wrong success code (but still 2xx, so should parse)
        message: "OK".to_string(),
        lines: vec!["local Local to this news server.".to_string()],
    };

    // Should still parse because is_success() checks 2xx
    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 1);
}

#[test]
fn test_parse_list_distributions_response_malformed_line() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "local Local to this server.".to_string(),
            "nodescription".to_string(), // Missing description - should be skipped
            "usa United States.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    // Malformed line should be skipped
    assert_eq!(distributions.len(), 2);
    assert_eq!(distributions[0].name, "local");
    assert_eq!(distributions[1].name, "usa");
}

#[test]
fn test_parse_list_distributions_response_empty_description() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "local ".to_string(), // Empty description (just whitespace)
            "usa Distribution area.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 2);
    assert_eq!(distributions[0].name, "local");
    assert_eq!(distributions[0].description, "");
    assert_eq!(distributions[1].name, "usa");
}

#[test]
fn test_parse_list_distributions_response_extra_whitespace() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "local    Local to this server.".to_string(), // Extra spaces
            "usa\t\tUnited States area.".to_string(),     // Tabs
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 2);
    assert_eq!(distributions[0].name, "local");
    assert_eq!(distributions[0].description, "Local to this server.");
    assert_eq!(distributions[1].name, "usa");
    assert_eq!(distributions[1].description, "United States area.");
}

#[test]
fn test_parse_list_distributions_response_special_chars() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "local Local (city-wide).".to_string(),
            "usa U.S.A. - 50 states & territories.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 2);
    assert_eq!(distributions[0].description, "Local (city-wide).");
    assert_eq!(
        distributions[1].description,
        "U.S.A. - 50 states & territories."
    );
}

#[test]
fn test_parse_list_distributions_response_long_description() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "world Articles distributed to all servers worldwide across multiple continents and countries.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 1);
    assert_eq!(distributions[0].name, "world");
    assert!(distributions[0].description.len() > 50);
}

// RFC 6048 Section 4 Examples

#[test]
fn test_rfc6048_section4_example() {
    // Example from RFC 6048 Section 4
    let response = NntpResponse {
        code: 215,
        message: "List of distributions follows".to_string(),
        lines: vec![
            "fr Local to France.".to_string(),
            "local Local to this news server.".to_string(),
            "thissite Local to this site.".to_string(),
            "usa Local to the United States of America.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 4);

    assert_eq!(distributions[0].name, "fr");
    assert_eq!(distributions[0].description, "Local to France.");

    assert_eq!(distributions[1].name, "local");
    assert_eq!(distributions[1].description, "Local to this news server.");

    assert_eq!(distributions[2].name, "thissite");
    assert_eq!(distributions[2].description, "Local to this site.");

    assert_eq!(distributions[3].name, "usa");
    assert_eq!(
        distributions[3].description,
        "Local to the United States of America."
    );
}

// Real-World Scenarios

#[test]
fn test_parse_list_distributions_response_typical_server() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "Distributions list".to_string(),
        lines: vec![
            "local Local distribution".to_string(),
            "regional Regional distribution".to_string(),
            "national National distribution".to_string(),
            "world Worldwide distribution".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 4);
    assert_eq!(distributions[0].name, "local");
    assert_eq!(distributions[1].name, "regional");
    assert_eq!(distributions[2].name, "national");
    assert_eq!(distributions[3].name, "world");
}

#[test]
fn test_parse_list_distributions_response_geographic() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "de Germany and German-speaking areas.".to_string(),
            "uk United Kingdom distribution.".to_string(),
            "eu European Union member states.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 3);
    assert_eq!(distributions[0].name, "de");
    assert_eq!(distributions[1].name, "uk");
    assert_eq!(distributions[2].name, "eu");
}

#[test]
fn test_parse_list_distributions_response_organizational() {
    let response = NntpResponse {
        code: codes::LIST_INFORMATION_FOLLOWS,
        message: "List follows".to_string(),
        lines: vec![
            "company Internal company distribution.".to_string(),
            "campus University campus only.".to_string(),
        ],
    };

    let distributions = commands::parse_list_distributions_response(response).unwrap();
    assert_eq!(distributions.len(), 2);
    assert_eq!(distributions[0].name, "company");
    assert_eq!(
        distributions[0].description,
        "Internal company distribution."
    );
    assert_eq!(distributions[1].name, "campus");
    assert_eq!(distributions[1].description, "University campus only.");
}

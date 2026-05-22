use std::collections::HashSet;

use crate::{AppError, ErrorCode};

pub fn validate_district_codes(
    district_codes: &[String],
    valid_code_set: &HashSet<String>,
) -> Result<(), AppError> {
    let mut seen: HashSet<&str> = HashSet::new();

    for code in district_codes {
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("invalid district code format: {code}"),
            ));
        }

        if !seen.insert(code.as_str()) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("duplicate district code: {code}"),
            ));
        }

        if !valid_code_set.contains(code) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("unknown district code: {code}"),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::Value;

    use super::validate_district_codes;

    fn load_valid_code_set() -> HashSet<String> {
        let raw = include_str!("../../../docs/jxemall-district-code-name-map.json");
        let map: Value = serde_json::from_str(raw).expect("district code map should be valid json");
        let obj = map
            .as_object()
            .expect("district code map should be a json object");

        obj.keys().cloned().collect()
    }

    fn to_codes(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn single_selection_cases_should_pass() {
        let valid_codes = load_valid_code_set();
        let cases = vec![
            to_codes(&["360102"]),
            to_codes(&["360699"]),
            to_codes(&["360992"]),
            to_codes(&["361209"]),
            to_codes(&["980701"]),
            to_codes(&["369900"]),
            to_codes(&["360622"]),
        ];

        for input in cases {
            validate_district_codes(&input, &valid_codes)
                .expect("single-selection case should be valid");
        }
    }

    #[test]
    fn same_city_multi_selection_cases_should_pass() {
        let valid_codes = load_valid_code_set();
        let cases = vec![
            to_codes(&[
                "360102", "360103", "360104", "360105", "360111", "360112", "360121", "360123",
                "360124", "360192", "360193", "360194", "360199",
            ]),
            to_codes(&[
                "360300", "360302", "360313", "360321", "360322", "360323", "360391", "360399",
            ]),
            to_codes(&["360403", "360404", "360423"]),
            to_codes(&["360591", "360592", "360599"]),
            to_codes(&["360791", "360792"]),
        ];

        for input in cases {
            validate_district_codes(&input, &valid_codes)
                .expect("same-city multi-selection case should be valid");
        }
    }

    #[test]
    fn cross_city_multi_selection_cases_should_pass() {
        let valid_codes = load_valid_code_set();
        let cases = vec![
            to_codes(&["360199", "360499", "361099"]),
            to_codes(&["360292", "360693", "360991", "361209"]),
            to_codes(&["360103", "360104", "360321", "360322", "360803", "360821"]),
            to_codes(&["360103", "360104", "360321", "360322", "360803", "360821", "360899"]),
            to_codes(&["360103", "360104", "360321", "360322", "360803", "360821", "361099"]),
            to_codes(&["360103", "360104", "360321", "360322", "360803", "360821", "360892"]),
            to_codes(&["360103", "360104", "360321", "360322", "360803", "360821", "361192"]),
            to_codes(&[
                "360103", "360104", "360321", "360322", "360803", "360821", "361192", "361199",
            ]),
            to_codes(&[
                "360103", "360104", "360321", "360322", "360803", "360821", "360892", "360899",
            ]),
            to_codes(&[
                "360103", "360104", "360321", "360322", "360803", "360821", "360892", "360899",
                "369900",
            ]),
            to_codes(&[
                "360103", "360104", "360321", "360322", "360803", "360821", "360892", "369900",
            ]),
            to_codes(&[
                "360103", "360104", "360321", "360322", "360803", "360821", "360892", "361192",
                "369900",
            ]),
            to_codes(&[
                "360103", "360203", "360222", "360404", "360622", "361100", "361102", "361103",
                "361121", "361123", "361124", "361125", "361126", "361127", "361128", "361129",
                "361130", "361181", "361191", "361192", "361193", "361199",
            ]),
        ];

        for input in cases {
            validate_district_codes(&input, &valid_codes)
                .expect("cross-city multi-selection case should be valid");
        }
    }

    #[test]
    fn duplicate_code_should_fail() {
        let valid_codes = load_valid_code_set();
        let input = to_codes(&["360103", "360103", "360321", "360322"]);

        let err = validate_district_codes(&input, &valid_codes)
            .expect_err("duplicate code case should fail");
        assert!(err.message.contains("duplicate district code"));
    }

    #[test]
    fn unknown_code_should_fail() {
        let valid_codes = load_valid_code_set();
        let input = to_codes(&["369998"]);

        let err = validate_district_codes(&input, &valid_codes)
            .expect_err("unknown code case should fail");
        assert!(err.message.contains("unknown district code"));
    }

    #[test]
    fn invalid_format_should_fail() {
        let valid_codes = load_valid_code_set();
        let input = to_codes(&["36A103"]);

        let err = validate_district_codes(&input, &valid_codes)
            .expect_err("invalid format case should fail");
        assert!(err.message.contains("invalid district code format"));
    }
}

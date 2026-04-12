#[cfg(test)]
mod tests {
    use crate::models::role::Role;
    use std::str::FromStr;

    #[test]
    fn parse_titipers() {
        assert_eq!(Role::from_str("TITIPERS").unwrap(), Role::Titipers);
    }

    #[test]
    fn parse_jastiper() {
        assert_eq!(Role::from_str("JASTIPER").unwrap(), Role::Jastiper);
    }

    #[test]
    fn parse_admin() {
        assert_eq!(Role::from_str("ADMIN").unwrap(), Role::Admin);
    }

    #[test]
    fn parse_system() {
        assert_eq!(Role::from_str("SYSTEM").unwrap(), Role::System);
    }

    #[test]
    fn parse_lowercase_returns_err() {
        assert!(Role::from_str("titipers").is_err());
        assert!(Role::from_str("admin").is_err());
    }

    #[test]
    fn parse_unknown_returns_err() {
        assert!(Role::from_str("SUPERUSER").is_err());
        assert!(Role::from_str("").is_err());
    }

    #[test]
    fn display_titipers() {
        assert_eq!(Role::Titipers.to_string(), "TITIPERS");
    }

    #[test]
    fn display_jastiper() {
        assert_eq!(Role::Jastiper.to_string(), "JASTIPER");
    }

    #[test]
    fn display_admin() {
        assert_eq!(Role::Admin.to_string(), "ADMIN");
    }

    #[test]
    fn display_system() {
        assert_eq!(Role::System.to_string(), "SYSTEM");
    }

    #[test]
    fn as_str_consistent_with_display() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for r in &roles {
            assert_eq!(r.as_str(), r.to_string().as_str());
        }
    }

    #[test]
    fn roundtrip_all_roles() {
        let roles = [Role::Titipers, Role::Jastiper, Role::Admin, Role::System];
        for r in &roles {
            let parsed = Role::from_str(r.as_str()).expect("roundtrip gagal");
            assert_eq!(&parsed, r);
        }
    }

    #[test]
    fn clone_equals_original() {
        let r = Role::Jastiper;
        assert_eq!(r.clone(), r);
    }

    #[test]
    fn different_roles_not_equal() {
        assert_ne!(Role::Admin, Role::Titipers);
        assert_ne!(Role::System, Role::Jastiper);
    }
}

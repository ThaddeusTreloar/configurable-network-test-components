use crate::config::ListenerRuleConfiguration;

pub struct ListenerRule {
    pub target_group: String,
    pub path_prefix: String,
    pub path_rewrite: String,
}

impl From<ListenerRuleConfiguration> for ListenerRule {
    fn from(
        ListenerRuleConfiguration {
            target_group,
            path_prefix: raw_prefix,
            path_rewrite: raw_rewrite,
        }: ListenerRuleConfiguration,
    ) -> Self {
        let path_prefix = format!(
            "/{}",
            raw_prefix.trim_start_matches("/").trim_end_matches("/")
        );
        let path_rewrite = format!(
            "/{}",
            raw_rewrite.trim_start_matches("/").trim_end_matches("/")
        );

        Self {
            target_group,
            path_prefix,
            path_rewrite,
        }
    }
}

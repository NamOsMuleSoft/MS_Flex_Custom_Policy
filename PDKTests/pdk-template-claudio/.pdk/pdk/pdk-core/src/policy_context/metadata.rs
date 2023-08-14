// Copyright 2023 Salesforce, Inc. All rights reserved.
use std::collections::BTreeMap;

use crate::host::property::PropertyAccessor;
use serde::Deserialize;
use url::{Host, Url};

const PLUGIN_NAME: &[&str] = &["plugin_name"];
const DEFAULT_POLICY_ID: &str = "NoPolicyId";
const DEFAULT_POLICY_NAMESPACE: &str = "NoPolicyNamespace";
const DEFAULT_FLEX_NAME: &str = "NoFlexName";

#[derive(Clone, Hash)]
pub struct PolicyMetadata {
    flex_name: String,
    policy_id: String,
    policy_namespace: String,
    context: ApiContext,
}

#[derive(Clone, Deserialize, Debug, Default, Hash)]
pub struct Api {
    #[serde(rename = "id")]
    id: String,

    #[serde(rename = "name")]
    name: String,

    #[serde(rename = "legacyApiId")]
    legacy_api_id: String,

    #[serde(rename = "version")]
    version: String,
}

impl Api {
    pub fn new(id: String, name: String, legacy_api_id: String, version: String) -> Self {
        Api {
            id,
            name,
            legacy_api_id,
            version,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn legacy_api_id(&self) -> &str {
        &self.legacy_api_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

#[derive(Clone, Deserialize, Debug, Default, Hash)]
pub struct ApiContext {
    #[serde(rename = "policyConfig")]
    pub policy_config: Option<PolicyConfig>,

    #[serde(rename = "api")]
    api: Option<Api>,

    #[serde(rename = "tiers")]
    tiers: Option<Vec<ApiSla>>,

    #[serde(rename = "identityManagement")]
    identity_management: Option<IdentityManagementContext>,

    #[serde(rename = "environment")]
    environment: Option<EnvironmentContext>,

    #[serde(rename = "platformPolicyIDs")]
    platform_policy_ids: Option<BTreeMap<String, String>>,
}

impl ApiContext {
    pub fn new(
        policy_config: Option<PolicyConfig>,
        api: Option<Api>,
        tiers: Option<Vec<ApiSla>>,
        identity_management: Option<IdentityManagementContext>,
        environment: Option<EnvironmentContext>,
        platform_policy_ids: Option<BTreeMap<String, String>>,
    ) -> Self {
        ApiContext {
            policy_config,
            api,
            tiers,
            identity_management,
            environment,
            platform_policy_ids,
        }
    }

    pub fn api(&self) -> Option<&Api> {
        self.api.as_ref()
    }

    pub fn tiers(&self) -> Option<&Vec<ApiSla>> {
        self.tiers.as_ref()
    }

    pub fn identity_management(&self) -> Option<&IdentityManagementContext> {
        self.identity_management.as_ref()
    }

    pub fn environment(&self) -> Option<&EnvironmentContext> {
        self.environment.as_ref()
    }

    pub fn platform_policy_ids(&self) -> Option<&BTreeMap<String, String>> {
        self.platform_policy_ids.as_ref()
    }
}

#[derive(Deserialize, Debug, Default, Clone, Hash)]
pub struct PolicyConfig {
    logging: Option<Logging>,
}

impl PolicyConfig {
    pub fn logging(&self) -> Option<&Logging> {
        self.logging.as_ref()
    }
}

#[derive(Deserialize, Debug, Default, Clone, Hash)]
pub struct Logging {
    level: String,
}

impl Logging {
    pub fn level(&self) -> &str {
        self.level.as_ref()
    }
}

#[derive(Deserialize, Debug, Default, Clone, Hash)]
pub struct ApiSla {
    #[serde(rename = "id")]
    id: String,

    #[serde(rename = "limits")]
    tiers: Vec<Tier>,
}

impl ApiSla {
    pub fn new(id: String, tiers: Vec<Tier>) -> Self {
        ApiSla { id, tiers }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn tiers(&self) -> &Vec<Tier> {
        &self.tiers
    }
}

#[derive(Deserialize, Debug, Default, Clone, Hash)]
pub struct Tier {
    #[serde(rename = "maximumRequests")]
    requests: u64,

    #[serde(rename = "timePeriodInMilliseconds")]
    period_in_millis: u64,
}

impl Tier {
    pub fn new(requests: u64, period_in_millis: u64) -> Self {
        Tier {
            requests,
            period_in_millis,
        }
    }

    pub fn requests(&self) -> u64 {
        self.requests
    }

    pub fn period_in_millis(&self) -> u64 {
        self.period_in_millis
    }
}

#[derive(Clone, Deserialize, Debug, Default, Hash)]
pub struct IdentityManagementContext {
    #[serde(rename = "clientId")]
    client_id: String,

    #[serde(rename = "clientSecret")]
    client_secret: String,

    #[serde(rename = "tokenUrl")]
    token_url: String,

    #[serde(rename = "serviceName")]
    service_name: String,
}

impl IdentityManagementContext {
    pub fn new(
        client_id: String,
        client_secret: String,
        token_url: String,
        service_name: String,
    ) -> Self {
        IdentityManagementContext {
            client_id,
            client_secret,
            token_url,
            service_name,
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    pub fn token_url(&self) -> &str {
        &self.token_url
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

#[derive(Deserialize, Debug, Default, Clone, Hash)]
pub struct EnvironmentContext {
    #[serde(rename = "organizationId")]
    organization_id: String,

    #[serde(rename = "environmentId")]
    environment_id: String,

    #[serde(rename = "masterOrganizationId")]
    root_organization_id: String,

    #[serde(rename = "clusterId")]
    cluster_id: String,

    #[serde(rename = "anypoint")]
    anypoint: Option<AnypointContext>,
}

impl EnvironmentContext {
    pub fn new(
        organization_id: String,
        environment_id: String,
        root_organization_id: String,
        cluster_id: String,
        anypoint: Option<AnypointContext>,
    ) -> Self {
        EnvironmentContext {
            organization_id,
            environment_id,
            root_organization_id,
            cluster_id,
            anypoint,
        }
    }

    pub fn organization_id(&self) -> &str {
        &self.organization_id
    }

    pub fn environment_id(&self) -> &str {
        &self.environment_id
    }

    pub fn master_organization_id(&self) -> &str {
        &self.root_organization_id
    }

    pub fn cluster_id(&self) -> &str {
        &self.cluster_id
    }

    pub fn anypoint(&self) -> Option<&AnypointContext> {
        // TODO: W-10875672: This is to avoid the runtime warning when parsing. Remove once MGW_DATASOURCE_PLATFORM_ENABLED is set by default.
        match self.anypoint.as_ref() {
            None => None,
            Some(a) => {
                if a.service_name.is_some() && a.url.is_some() {
                    Some(a)
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Deserialize, Debug, Default, Clone, Hash)]
pub struct AnypointContext {
    #[serde(rename = "clientId")]
    client_id: String,

    #[serde(rename = "clientSecret")]
    client_secret: String,

    // TODO: W-10875672: Remove option once MGW_DATASOURCE_PLATFORM_ENABLED is set by default.
    #[serde(rename = "serviceName")]
    service_name: Option<String>,

    // TODO: W-10875672: Remove option once MGW_DATASOURCE_PLATFORM_ENABLED is set by default.
    #[serde(rename = "url")]
    url: Option<String>,
}

impl AnypointContext {
    pub fn new(
        client_id: String,
        client_secret: String,
        service_name: String,
        url: String,
    ) -> Self {
        AnypointContext {
            client_id,
            client_secret,
            service_name: Some(service_name),
            url: Some(url),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    pub fn service_name(&self) -> &str {
        self.service_name.as_deref().unwrap_or("UNDEFINED")
    }

    // TODO: W-10875672: Remove once MGW_DATASOURCE_PLATFORM_ENABLED is set by default.
    fn url(&self) -> &str {
        self.url.as_deref().unwrap_or("UNDEFINED")
    }

    pub fn base_path(&self) -> String {
        Url::parse(self.url())
            .map(|url| url.path().to_string())
            .unwrap_or_else(|_| "/".to_string())
    }

    pub fn authority(&self) -> String {
        Url::parse(self.url())
            .ok()
            .and_then(Self::get_host)
            .unwrap_or_else(|| "anypoint.com".to_string())
    }

    fn get_host(url: Url) -> Option<String> {
        let host = url.host()?;
        match host {
            Host::Domain(host) => Some(host.to_string()),
            _ => None,
        }
    }
}

fn read_string(property_accessor: &dyn PropertyAccessor, coordinate: &[&str]) -> Option<String> {
    property_accessor.read_property(coordinate).and_then(|bytes| {
        std::str::from_utf8(bytes.as_slice())
            .ok()
            .map(str::to_string)
    })
}

pub fn read_api_name_from_plugin_name(property_accessor: &dyn PropertyAccessor) -> String {
    read_string(property_accessor, PLUGIN_NAME)
        .map(|name| PolicyMetadata::split_plugin_name(&name).unwrap_or_default())
        .map(|(api_id, _, _)| api_id)
        .unwrap_or_default()
}

impl PolicyMetadata {
    /// Parses the policy id, policy namespace and api id from the plugin name.
    ///
    /// The expected format for this parsing to be ok is
    ///     <policy id>.<policy namespace>.<api id>
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - A string slice holding the plugin name with the expected format
    ///
    /// # Return value
    ///
    /// A Result holding a 3-tuple with the three values as Strings if the parsing was successful,
    /// an string describing the error otherwise.
    pub fn split_plugin_name(plugin_name: &str) -> Result<(String, String, String), String> {
        log::debug!("Plugin name: {}", plugin_name);
        let parts: Vec<&str> = plugin_name.split('.').collect();
        if parts.len() >= 3 {
            let mut iter = parts.iter();
            let policy_id = iter.next().unwrap().to_string();
            let policy_namespace = iter.next().unwrap().to_string();
            let mut api_id = iter.next().unwrap().to_string();
            let mut aux = iter.next();
            while aux.is_some() {
                api_id = format!("{}.{}", api_id, aux.unwrap());
                aux = iter.next();
            }
            Ok((api_id, policy_namespace, policy_id))
        } else {
            Err(format!(
                "Plugin name '{:?}' did not match the expected format",
                plugin_name
            ))
        }
    }

    pub fn from(property_accessor: &dyn PropertyAccessor) -> Self {
        let flex_name = read_string(property_accessor, &["node", "id"]).unwrap_or_else(|| {
            log::debug!("did not find nodeId");
            DEFAULT_FLEX_NAME.to_string()
        });
        read_string(property_accessor, PLUGIN_NAME)
            .map(
                |name: String| match Self::split_plugin_name(name.as_str()) {
                    Ok((api_id, policy_namespace, policy_id)) => {
                        let api_context = ApiContext::from(property_accessor, api_id.as_ref());
                        Self::new(flex_name, policy_id, policy_namespace, api_context)
                    }
                    Err(message) => {
                        log::warn!("{} (ErrorCode: FLTR-201).", message);
                        Self::new(
                            flex_name,
                            name,
                            DEFAULT_POLICY_NAMESPACE.to_string(),
                            ApiContext::default(),
                        )
                    }
                },
            )
            .unwrap_or_else(Self::default)
    }

    pub fn new(
        flex_name: String,
        policy_id: String,
        policy_namespace: String,
        context: ApiContext,
    ) -> Self {
        Self {
            flex_name,
            policy_id,
            policy_namespace,
            context,
        }
    }

    pub fn policy_id(&self) -> &str {
        self.policy_id.as_ref()
    }

    pub fn policy_namespace(&self) -> &str {
        self.policy_namespace.as_ref()
    }

    pub fn policy_name(&self) -> &str {
        // TODO: Change this to use proper Policy Name
        self.policy_id.as_ref()
    }

    pub fn api_info(&self) -> Option<&Api> {
        self.context.api.as_ref()
    }

    pub fn anypoint_environment(&self) -> Option<&EnvironmentContext> {
        self.context.environment.as_ref()
    }

    pub fn api_tiers(&self) -> Option<&Vec<ApiSla>> {
        self.context.tiers.as_ref()
    }

    pub fn flex_name(&self) -> &str {
        self.flex_name.as_str()
    }
    pub fn identity_management_context(&self) -> Option<&IdentityManagementContext> {
        self.context.identity_management.as_ref()
    }

    pub fn policy_config(&self) -> Option<&PolicyConfig> {
        self.context.policy_config.as_ref()
    }

    pub fn platform_policy_ids(&self) -> Option<&BTreeMap<String, String>> {
        self.context.platform_policy_ids.as_ref()
    }
}

impl Default for PolicyMetadata {
    fn default() -> Self {
        Self {
            flex_name: DEFAULT_FLEX_NAME.to_string(),
            policy_id: DEFAULT_POLICY_ID.to_string(),
            policy_namespace: DEFAULT_POLICY_NAMESPACE.to_string(),
            context: ApiContext::default(),
        }
    }
}

impl ApiContext {
    pub fn from(property_accessor: &dyn PropertyAccessor, api_instance_name: &str) -> Self {
        match read_string(
            property_accessor,
            &[
                "listener_metadata",
                "filter_metadata",
                api_instance_name,
                "context",
            ],
        ) {
            None => {
                log::debug!(
                    "Api context info for '{:?}' was not present.",
                    api_instance_name
                );
                ApiContext::default()
            }
            Some(value) => {
                log::debug!("Api context for {} successfully parsed.", api_instance_name);
                match serde_json::from_str(value.as_str()) {
                    Ok(context) => context,
                    Err(_cause) => {
                        log::warn!("Could not parse context info: incomplete/malformed incoming data from platform (ErrorCode: FLTR-201).");
                        ApiContext::default()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use classy::proxy_wasm::types::Bytes;
    use mockall::mock;

    use crate::policy_context::metadata::{
        PolicyMetadata, DEFAULT_POLICY_ID, DEFAULT_POLICY_NAMESPACE,
    };

    mock! {
        pub PropertyAccessor {}
        impl crate::host::property::PropertyAccessor for PropertyAccessor{
                fn read_property<'a>(&self, path: &[&'a str]) -> Option<Bytes>;
                fn set_property<'a>(&self, path: &[&'a str], value: &[u8]);
        }
    }

    #[test]
    pub fn full_api_info() {
        let mut property_accessor = MockPropertyAccessor::new();
        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        expect_stream_metadata(&mut property_accessor);

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(metadata.flex_name(), "my-flex-name");
        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "error"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_some());
        assert!(metadata.identity_management_context().is_some());
        assert!(metadata.api_info().is_some());

        let api_info = metadata.api_info().unwrap();
        assert_eq!(api_info.id(), "anypoint_id");
        assert_eq!(api_info.name(), "anypoint_name");
        assert_eq!(api_info.legacy_api_id(), "anypoint_legacy_api_id");
        assert_eq!(api_info.version(), "anypoint_version");

        let tiers = metadata.api_tiers().unwrap();
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers.get(0).unwrap().id(), "123");
        assert_eq!(tiers.get(0).unwrap().tiers().get(0).unwrap().requests, 100);
        assert_eq!(
            tiers
                .get(0)
                .unwrap()
                .tiers()
                .get(0)
                .unwrap()
                .period_in_millis,
            10
        );
        assert_eq!(tiers.get(0).unwrap().tiers().get(1).unwrap().requests, 1);
        assert_eq!(
            tiers
                .get(0)
                .unwrap()
                .tiers()
                .get(1)
                .unwrap()
                .period_in_millis,
            5
        );

        let identity = metadata.identity_management_context().unwrap();
        assert_eq!(identity.client_id(), "myClientId");
        assert_eq!(identity.client_secret(), "myClientSecret");
        assert_eq!(identity.token_url(), "myTokenUrl");
        assert_eq!(identity.service_name(), "myServiceName");

        let environment = metadata.anypoint_environment().unwrap();
        assert_eq!(environment.organization_id(), "some_org_id");
        assert_eq!(environment.environment_id(), "some_env_id");
        assert_eq!(environment.master_organization_id(), "some_root_org_id");
        assert_eq!(environment.cluster_id(), "some_cluster_id");

        assert!(environment.anypoint().is_some());
        let anypoint = environment.anypoint().unwrap();
        assert_eq!(anypoint.client_id(), "some_client_id");
        assert_eq!(anypoint.client_secret(), "some_secret");
        assert_eq!(anypoint.service_name(), "some_service_name");
        assert_eq!(anypoint.base_path(), "/path");
        assert_eq!(anypoint.authority(), "qax.anypoint.mulesoft.com");

        let policy_ids = metadata.platform_policy_ids().unwrap();
        assert_eq!(policy_ids["first_policy"], "first_policy_platform_id");
        assert_eq!(policy_ids["second_policy"], "second_policy_platform_id");
    }

    fn expect_stream_metadata(property_accessor: &mut MockPropertyAccessor) {
        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| Some(FULL_API_INFO.as_bytes().to_vec()));
    }

    fn expect_stream_plugin_name(property_accessor: &mut MockPropertyAccessor) {
        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| *arg == ["plugin_name"])
            .times(1)
            .returning(|_| Some("123-123.rate-limit.456456".as_bytes().to_vec()));
    }

    fn expect_stream_node_id(property_accessor: &mut MockPropertyAccessor) {
        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| *arg == ["node", "id"])
            .times(1)
            .returning(|_| Some("my-flex-name".as_bytes().to_vec()));
    }

    #[test]
    pub fn no_context_info() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .return_const(None);

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.policy_config().is_none());
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
        assert!(metadata.anypoint_environment().is_none());
    }

    #[test]
    pub fn no_tiers() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
            {
                "policyConfig": {
                    "logging": {
                        "level": "warn"
                      }
                 },
               "api": {
                "id": "anypoint_id",
                "name": "anypoint_name",
                "legacyApiId": "anypoint_legacy_api_id",
                "version": "anypoint_version"
              },
              "identityManagement": {
                 "clientId": "myClientId",
                 "clientSecret": "myClientSecret",
                 "tokenUrl": "myTokenUrl",
                 "serviceName": "myServiceName"
              },
              "environment": {
                "organizationId": "some_org_id",
                "environmentId": "some_env_id",
                "masterOrganizationId": "some_root_org_id",
                "clusterId": "some_cluster_id",
                "anypoint" : {
                    "clientId": "some_client_id",
                    "clientSecret": "some_secret",
                    "serviceName": "some_service_name",
                    "url": "https://qax.anypoint.mulesoft.com/path"
                }
              }
            }"#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "warn"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_some());
        assert!(metadata.anypoint_environment().is_some());
        assert!(metadata
            .anypoint_environment()
            .unwrap()
            .anypoint()
            .is_some());
        assert!(metadata.api_info().is_some());
    }

    #[test]
    pub fn no_identity() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
            {
                "policyConfig": {
                    "logging": {
                        "level": "error"
                      }
                 },
              "api": {
                "id": "anypoint_id",
                "name": "anypoint_name",
                "legacyApiId": "anypoint_legacy_api_id",
                "version": "anypoint_version"
              },
              "tiers": [{
                "id": "123",
                "limits": [
                  {
                    "timePeriodInMilliseconds": 10,
                    "maximumRequests": 100
                  },
                  {
                    "timePeriodInMilliseconds": 5,
                    "maximumRequests": 1
                  }
                ]
              }],
              "environment": {
                "organizationId": "some_org_id",
                "environmentId": "some_env_id",
                "masterOrganizationId": "some_root_org_id",
                "clusterId": "some_cluster_id",
                "anypoint": {
                    "clientId": "some_client_id",
                    "clientSecret": "some_secret",
                    "serviceName": "some_service_name",
                    "url": "https://qax.anypoint.mulesoft.com/path"
                }
              }
            }"#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "error"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_some());
        assert!(metadata.identity_management_context().is_none());
        assert!(metadata.api_info().is_some());
        assert!(metadata.anypoint_environment().is_some());
        assert!(metadata
            .anypoint_environment()
            .unwrap()
            .anypoint()
            .is_some());
    }

    #[test]
    pub fn no_api_info() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
                    {
                        "policyConfig": {
                            "logging": {
                                "level": "error"
                              }
                         },
                      "tiers": [{
                        "id": "123",
                        "limits": [
                          {
                            "timePeriodInMilliseconds": 10,
                            "maximumRequests": 100
                          },
                          {
                            "timePeriodInMilliseconds": 5,
                            "maximumRequests": 1
                          }
                        ]
                      }],
                      "identityManagement": {
                         "clientId": "myClientId",
                         "clientSecret": "myClientSecret",
                         "tokenUrl": "myTokenUrl",
                         "serviceName": "myServiceName"
                      },
                      "environment": {
                        "organizationId": "some_org_id",
                        "environmentId": "some_env_id",
                        "masterOrganizationId": "some_root_org_id",
                        "clusterId": "some_cluster_id",
                        "anypoint" : {
                            "clientId": "some_client_id",
                            "clientSecret": "some_secret",
                            "serviceName": "some_service_name",
                            "url": "https://qax.anypoint.mulesoft.com/path"
                        }
                      }
                    }
                    "#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "error"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_some());
        assert!(metadata.identity_management_context().is_some());
        assert!(metadata.anypoint_environment().is_some());
        assert!(metadata
            .anypoint_environment()
            .unwrap()
            .anypoint()
            .is_some());
        assert!(metadata.api_info().is_none());
    }

    #[test]
    pub fn no_anypoint_environment() {
        let mut property_accessor = MockPropertyAccessor::new();
        expect_stream_node_id(&mut property_accessor);

        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
                    {
                       "policyConfig": {
                          "logging": {
                            "level": "error"
                          }
                        },
                       "api": {
                        "id": "anypoint_id",
                        "name": "anypoint_name",
                        "legacyApiId": "anypoint_legacy_api_id",
                        "version": "anypoint_version"
                      },
                      "tiers": [{
                        "id": "123",
                        "limits": [
                          {
                            "timePeriodInMilliseconds": 10,
                            "maximumRequests": 100
                          },
                          {
                            "timePeriodInMilliseconds": 5,
                            "maximumRequests": 1
                          }
                        ]
                      }],
                      "identityManagement": {
                         "clientId": "myClientId",
                         "clientSecret": "myClientSecret",
                         "tokenUrl": "myTokenUrl",
                         "serviceName": "myServiceName"
                      }
                    }
                    "#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "error"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_some());
        assert!(metadata.identity_management_context().is_some());
        assert!(metadata.anypoint_environment().is_none());
        assert!(metadata.api_info().is_some());
    }

    #[test]
    pub fn no_policy_ids() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
                    {
                        "api": {
                            "id": "anypoint_id",
                            "name": "anypoint_name",
                            "legacyApiId": "anypoint_legacy_api_id",
                            "version": "anypoint_version"
                          },
                         "tiers": [{
                            "id": "123",
                            "limits": [
                              {
                                "timePeriodInMilliseconds": 10,
                                "maximumRequests": 100
                              },
                              {
                                "timePeriodInMilliseconds": 5,
                                "maximumRequests": 1
                              }
                            ]
                          }],
                         "identityManagement": {
                            "clientId": "myClientId",
                            "clientSecret": "myClientSecret",
                            "tokenUrl": "myTokenUrl",
                            "serviceName": "myServiceName"
                         },
                         "environment": {
                           "organizationId": "some_org_id",
                           "environmentId": "some_env_id",
                           "masterOrganizationId": "some_root_org_id",
                           "clusterId": "some_cluster_id",
                           "anypoint" : {
                               "clientId": "some_client_id",
                               "clientSecret": "some_secret",
                               "serviceName": "some_service_name",
                               "url": "https://qax.anypoint.mulesoft.com/path"
                           }
                        }
                    }
                    "#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_some());
        assert!(metadata.identity_management_context().is_some());
        assert!(metadata.anypoint_environment().is_some());
        assert!(metadata.api_info().is_some());
        assert!(metadata.platform_policy_ids().is_none());
    }

    #[test]
    fn policy_binding_identity() {
        let mut property_accessor = MockPropertyAccessor::new();
        expect_stream_node_id(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| *arg == ["plugin_name"])
            .times(1)
            .returning(|_| {
                Some(
                    "some_binding_name.some_binding_namespace.456456"
                        .as_bytes()
                        .to_vec(),
                )
            });

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| None);

        let metadata = PolicyMetadata::from(&property_accessor);
        assert_eq!(metadata.policy_id(), "some_binding_name");
        assert_eq!(metadata.policy_namespace(), "some_binding_namespace");
    }

    #[test]
    pub fn no_plugin_name() {
        let mut property_accessor = MockPropertyAccessor::new();
        expect_stream_node_id(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| *arg == ["plugin_name"])
            .times(1)
            .return_const(None);

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(metadata.policy_id(), DEFAULT_POLICY_ID);
        assert_eq!(metadata.policy_namespace(), DEFAULT_POLICY_NAMESPACE);
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
    }

    #[test]
    pub fn not_matching_plugin_name() {
        let mut property_accessor = MockPropertyAccessor::new();
        expect_stream_node_id(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| *arg == ["plugin_name"])
            .times(1)
            .returning(|_| Some("123-123.456456".as_bytes().to_vec()));

        let metadata = PolicyMetadata::from(&property_accessor);

        assert!(metadata.policy_config().is_none());
        assert_eq!(metadata.policy_id(), "123-123.456456");
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
    }

    #[test]
    pub fn unexpected_json() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
            {
                "policyConfig": {
                    "logging": {
                        "level": "error"
                      }
                 },
              "tiers": [{
                "id": 123,
                "limits": [
                  {
                    "timePeriodInMilliseconds": 10,
                    "maximumRequests": 100
                  },
                  {
                    "timePeriodInMilliseconds": 5,
                    "maximumRequests": 1
                  }
                ]
              }],
              "identityManagement": {
                 "clientId": "myClientId",
                 "clientSecret": "myClientSecret",
                 "tokenUrl": "myTokenUrl",
                 "serviceName": "myServiceName"
              },
              "clientIdEnforcement": {
                "clientId": "myClientId",
                "clientSecret": "myClientSecret",
                "serviceName": "myServiceName",
                "orgId": "myOrgId",
                "envId": "myEnvId"
              }
            }"#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert!(metadata.policy_config().is_none());
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
    }

    #[test]
    pub fn only_api_info() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
                    {
                      "policyConfig": {
                        "logging": {
                            "level": "error"
                          }
                      },
                      "api": {
                        "id": "anypoint_id",
                        "name": "anypoint_name",
                        "legacyApiId": "anypoint_legacy_api_id",
                        "version": "anypoint_version"
                      }
                    }
                    "#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "error"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
        assert!(metadata.api_info().is_some());

        let api_info = metadata.api_info().unwrap();
        assert_eq!(api_info.id(), "anypoint_id");
        assert_eq!(api_info.name(), "anypoint_name");
        assert_eq!(api_info.legacy_api_id(), "anypoint_legacy_api_id");
        assert_eq!(api_info.version(), "anypoint_version");
    }

    #[test]
    pub fn only_environment_with_anypoint() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
                    {
                      "policyConfig": {
                        "logging": {
                            "level": "error"
                          }
                       },
                      "environment": {
                        "organizationId": "some_org_id",
                        "environmentId": "some_env_id",
                        "masterOrganizationId": "some_root_org_id",
                        "clusterId": "some_cluster_id",
                        "anypoint": {
                            "clientId": "some_client_id",
                            "clientSecret": "some_secret",
                            "serviceName": "some_service_name",
                            "url": "https://qax.anypoint.mulesoft.com/path"
                        }
                      }
                    }
                    "#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(
            metadata.policy_config().unwrap().logging().unwrap().level,
            "error"
        );
        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
        assert!(metadata.api_info().is_none());
        assert!(metadata.anypoint_environment().is_some());

        let environment = metadata.anypoint_environment().unwrap();
        assert_eq!(environment.organization_id(), "some_org_id");
        assert_eq!(environment.environment_id(), "some_env_id");
        assert_eq!(environment.master_organization_id(), "some_root_org_id");
        assert_eq!(environment.cluster_id(), "some_cluster_id");

        assert!(environment.anypoint().is_some());
        let anypoint = environment.anypoint().unwrap();
        assert_eq!(anypoint.client_id(), "some_client_id");
        assert_eq!(anypoint.client_secret(), "some_secret");
        assert_eq!(anypoint.service_name(), "some_service_name");
        assert_eq!(anypoint.base_path(), "/path");
        assert_eq!(anypoint.authority(), "qax.anypoint.mulesoft.com");
    }

    #[test]
    pub fn only_environment_without_anypoint() {
        let mut property_accessor = MockPropertyAccessor::new();

        expect_stream_node_id(&mut property_accessor);
        expect_stream_plugin_name(&mut property_accessor);

        property_accessor
            .expect_read_property()
            .withf(|arg: &[&str]| {
                *arg == ["listener_metadata", "filter_metadata", "456456", "context"]
            })
            .times(1)
            .returning(|_| {
                Some(
                    r#"
                    {
                      "environment": {
                        "organizationId": "some_org_id",
                        "environmentId": "some_env_id",
                        "masterOrganizationId": "some_root_org_id",
                        "clusterId": "some_cluster_id"
                      }
                    }
                    "#
                    .as_bytes()
                    .to_vec(),
                )
            });

        let metadata = PolicyMetadata::from(&property_accessor);

        assert_eq!(metadata.policy_id(), "123-123");
        assert!(metadata.api_tiers().is_none());
        assert!(metadata.identity_management_context().is_none());
        assert!(metadata.api_info().is_none());
        assert!(metadata.anypoint_environment().is_some());

        let environment = metadata.anypoint_environment().unwrap();
        assert_eq!(environment.organization_id(), "some_org_id");
        assert_eq!(environment.environment_id(), "some_env_id");
        assert_eq!(environment.master_organization_id(), "some_root_org_id");
        assert_eq!(environment.cluster_id(), "some_cluster_id");

        assert!(environment.anypoint().is_none());
    }

    const FULL_API_INFO: &str = r#"
            {
             "policyConfig": {
                "logging": {
                    "level": "error"
                  }
             },
             "api": {
                "id": "anypoint_id",
                "name": "anypoint_name",
                "legacyApiId": "anypoint_legacy_api_id",
                "version": "anypoint_version"
              },
              "tiers": [{
                "id": "123",
                "limits": [
                  {
                    "timePeriodInMilliseconds": 10,
                    "maximumRequests": 100
                  },
                  {
                    "timePeriodInMilliseconds": 5,
                    "maximumRequests": 1
                  }
                ]
              }],
              "identityManagement": {
                 "clientId": "myClientId",
                 "clientSecret": "myClientSecret",
                 "tokenUrl": "myTokenUrl",
                 "serviceName": "myServiceName"
              },
              "environment": {
                "organizationId": "some_org_id",
                "environmentId": "some_env_id",
                "masterOrganizationId": "some_root_org_id",
                "clusterId": "some_cluster_id",
                "anypoint" : {
                    "clientId": "some_client_id",
                    "clientSecret": "some_secret",
                    "serviceName": "some_service_name",
                    "url": "https://qax.anypoint.mulesoft.com/path"
                }
              },
              "platformPolicyIDs": {
               "first_policy": "first_policy_platform_id",
               "second_policy": "second_policy_platform_id"
              }
            }"#;
}

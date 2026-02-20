//! Example showing tenant-isolated config and secret access.

#[cfg(feature = "multi-tenant")]
use http_handle::tenant_isolation::{
    StaticSecretProvider, TenantConfig, TenantConfigStore, TenantId,
    TenantScopedSecrets,
};

fn main() {
    #[cfg(feature = "multi-tenant")]
    {
        let store = TenantConfigStore::default();
        let tenant = TenantId("acme".into());
        store
            .set_config(
                tenant.clone(),
                TenantConfig {
                    settings: [("mode".into(), "prod".into())]
                        .into_iter()
                        .collect(),
                },
            )
            .expect("set");
        let secret_provider = StaticSecretProvider::default()
            .with_secret(tenant.clone(), "db_password", "secret");
        let secrets = TenantScopedSecrets::new(secret_provider);
        let secret = secrets
            .read(&tenant, "db_password")
            .expect("read")
            .expect("present");
        println!("tenant secret length={}", secret.len());
    }
}

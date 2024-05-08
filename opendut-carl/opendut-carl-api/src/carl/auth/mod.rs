use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(any(feature = "client", feature = "oidc_client"))] {
        // used by CARL/EDGAR/CLEO
        pub mod error;
        pub mod auth_config;
        pub mod reqwest_client;
    }
}

cfg_if! {
    if #[cfg(feature = "client")] {
        // used by EDGAR/CLEO
        pub mod manager;
        pub mod service;
    }
}

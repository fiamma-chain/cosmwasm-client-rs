pub mod cosmos {
    pub mod base {
        pub mod query {
            pub mod v1beta1 {
                include!("cosmos.base.query.v1beta1.rs");
            }
        }
    }
}
pub mod babylon {
    pub mod btclightclient {
        pub mod v1 {
            include!("babylon.btclightclient.v1.rs");
        }
    }
}
pub mod cosmos_proto {
    include!("cosmos_proto.rs");
}
pub mod google {
    pub mod api {
        include!("google.api.rs");
    }
}
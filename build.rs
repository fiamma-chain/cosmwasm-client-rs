use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "src/generated";

    // ensure out_dir exists
    fs::create_dir_all(out_dir)?;

    // compile proto files
    tonic_build::configure().out_dir(out_dir).compile_protos(
        &[
            "proto/babylon/btclightclient/v1/query.proto",
            "proto/babylon/btclightclient/v1/params.proto",
        ],
        &["proto", "proto/third_party"],
    )?;

    // generate mod.rs file
    let mod_content = r#"

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

"#;

    fs::write(Path::new(out_dir).join("mod.rs"), mod_content.trim())?;

    Ok(())
}

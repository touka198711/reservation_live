use std::process::Command;

use tonic_build::Builder;

fn main() {
    // std::fs::remove_dir_all("src/pb");
    std::fs::create_dir_all("src/pb").unwrap();
    tonic_build::configure()
        .out_dir("src/pb")
        .with_sqlx_type(&["reservation.ReservationStatus"])
        .with_builder(&["reservation.ReservationQuery"])
        .with_builder_into_option("reservation.ReservationQuery", &["start", "end"])
        .with_builder_into(
            "reservation.ReservationQuery",
            &["resource_id", "user_id", "status", "desc"],
        )
        .field_attribute(
            "reservation.ReservationQuery.pagesize",
            "#[builder(setter(into), default = \"10\")]",
        )
        .field_attribute(
            "reservation.ReservationQuery.page",
            "#[builder(setter(into), default = \"1\")]",
        )
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();

    Command::new("cargo").args(&["fmt"]).output().unwrap();

    println!("cargo:rerun-if-changed=protos/reservation.proto");
    println!("cargo:rerun-if-changed=build.rs");
}

trait BuilderExt {
    fn with_sqlx_type(self, paths: &[&str]) -> Self;
    fn with_builder(self, paths: &[&str]) -> Self;
    fn with_builder_into(self, path: &str, fields: &[&str]) -> Self;
    fn with_builder_into_option(self, path: &str, fields: &[&str]) -> Self;
}

impl BuilderExt for Builder {
    fn with_sqlx_type(self, paths: &[&str]) -> Self {
        paths.iter().fold(self, |acc, path| {
            acc.type_attribute(path, "#[derive(sqlx::Type)]")
        })
    }

    fn with_builder(self, paths: &[&str]) -> Self {
        paths.iter().fold(self, |acc, path| {
            acc.type_attribute(path, "#[derive(derive_builder::Builder)]")
        })
    }

    fn with_builder_into(self, path: &str, fields: &[&str]) -> Self {
        fields.iter().fold(self, |acc, field| {
            acc.field_attribute(
                format!("{}.{}", path, field),
                "#[builder(setter(into), default)]",
            )
        })
    }

    fn with_builder_into_option(self, path: &str, fields: &[&str]) -> Self {
        fields.iter().fold(self, |acc, field| {
            acc.field_attribute(
                format!("{}.{}", path, field),
                "#[builder(setter(into, strip_option))]",
            )
        })
    }
}

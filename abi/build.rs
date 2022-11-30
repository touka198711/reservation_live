use std::process::Command;

fn main() {
    // std::fs::remove_dir_all("src/pb");
    std::fs::create_dir_all("src/pb").unwrap();
    tonic_build::configure()
        .out_dir("src/pb")
        .type_attribute("reservation.ReservationStatus", "#[derive(sqlx::Type)]")
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();

    Command::new("cargo").args(&["fmt"]).output().unwrap();

    println!("cargo:rerun-if-changed=protos/reservation.proto");
    println!("cargo:rerun-if-changed=build.rs");
}

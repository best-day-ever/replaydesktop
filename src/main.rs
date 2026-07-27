fn main() {
    let argv = match std::env::args_os()
        .map(|argument| argument.into_string())
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(argv) => argv,
        Err(_) => {
            eprint!(
                "replay-host-doctor: arguments must be valid UTF-8\n{}",
                replay_host_doctor::cli::public_usage()
            );
            std::process::exit(replay_host_doctor::DoctorExit::Usage.code());
        }
    };
    std::process::exit(replay_host_doctor::main_entry(argv).code());
}

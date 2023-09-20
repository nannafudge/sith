use sith::test_suite;

#[test_suite]
mod suite {
    use sith::test_case;

    #[setup]
    fn setup() {
        println!("SETUP!");
    }

    #[teardown]
    fn teardown() {
        println!("TEARDOWN!");
    }

    #[test_case]
    fn unparameterized() {
        println!("Hello from a unparameterized test!");
    }
}

#[test_suite]
mod other_suite {
    use sith::test_case;

    struct TestStruct(usize);

    // Imaginary seed value...
    const SEED: usize = 232523323;

    // Feel free to move these to another module
    fn setup_error_handler() {
        use std::io::Write;

        let handler = std::panic::take_hook();
        std::panic::set_hook(Box::new(move | info | {
            let _ = std::io::stderr().write_fmt(format_args!("failed with seed: {}\n", SEED));
            handler(info);
        }));
    }

    fn reset_error_handler() {
        let _ = std::panic::take_hook();
    }

    // Or even embed them directly into setup/teardown. I prefer this way, however
    #[setup]
    fn setup() {
        setup_error_handler();
    }

    #[teardown]
    fn teardown() {
        reset_error_handler();
    }

    #[test_case(with_zero, with(TestStruct(0)))]
    #[test_case(with_one, with(TestStruct(1)))]
    fn crucial_struct(my_struct: TestStruct) {
        assert_eq!(my_struct.0, 0);
    }
}

fn main() {}
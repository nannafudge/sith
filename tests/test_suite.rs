use sith::test_suite;

#[test_suite]
mod supports_rustc_test {
    static MEME: usize = INIT;
    static MEMETWO: usize = INIT;

    #[init]
    fn init() {
        MEME = 123;
        MEMETWO = 456;
    }

    #[setup]
    fn setup() {
        let has_ran_setup = true;
    }

    #[teardown]
    fn teardown() {
        assert!(post_setup);
    }

    #[test]
    fn inner() {
        assert!(has_ran_setup);
        let post_setup = true;

        println!("{}", MEME);
        println!("{}", MEMETWO);
    }
}


#[test_suite]
mod supports_sith_test_case {
    use sith::test_case;

    #[setup]
    fn setup() {
        let has_ran_setup = true;
    }

    #[teardown]
    fn teardown() {
        assert!(post_setup)
    }

    #[test_case]
    fn inner() {
        assert!(has_ran_setup);
        let post_setup = true;
    }
}

#[test_suite]
mod supports_wasm_bindgen_test {
    // Interanlly matches on name - no need
    // to test against *actual* wasm_bindgen_test impl
    use sith::test_case as wasm_bindgen_test;

    #[setup]
    fn setup() {
        let has_ran_setup = true;
    }

    #[teardown]
    fn teardown() {
        assert!(post_setup)
    }

    #[wasm_bindgen_test]
    fn inner() {
        assert!(has_ran_setup);
        let post_setup = true;
    }
}

#[test_suite]
mod ignores_empty_modules {
    
}
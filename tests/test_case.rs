#![allow(unnameable_test_items)]

use sith::test_case;

#[test]
fn unparameterized() {
    #[test_case]
    fn inner() {}

    inner();
}

mod name {
    use super::test_case;

    #[test]
    fn parameterized_argname_single() {
        #[test_case(named)]
        fn inner() {}
    
        inner_named();
    }
    
    #[test]
    fn parameterized_argname_multiple() {
        #[test_case(one)]
        #[test_case(two)]
        #[test_case(three)]
        fn inner() {}
    
        inner_one();
        inner_two();
        inner_three();
    }
    
    #[test]
    fn parameterized_argname_multiple_keywords() {
        #[test_case(for)]
        #[test_case(usize)]
        #[test_case(in)]
        #[test_case(loop)]
        fn inner() {}
    
        inner_for();
        inner_usize();
        inner_in();
        inner_loop();
    }
}

mod with {
    use super::test_case;

    #[test]
    fn parameterized_argwith() {
        #[test_case(with(0))]
        fn inner(val: usize) {
            assert_eq!(val, 0usize);
        }

        inner();
    }

    #[test]
    fn parameterized_argwith_multiple_inputs() {
        #[test_case(with(0, 1, 2))]
        fn inner(zero: usize, one: usize, two: usize) {
            assert_eq!((zero, one, two), (0usize, 1usize, 2usize));
        }

        inner();
    }

    #[test]
    fn parameterized_argwith_multiple_input_types() {
        #[test_case(with(0, 1, 2))]
        fn inner(zero: isize, one: usize, two: u8) {
            assert_eq!((zero, one, two), (0isize, 1usize, 2u8));
        }

        inner();
    }

    #[test]
    fn parameterized_argwith_multiple_args_and_cases() {
        #[test_case(zero, with(0, 0))]
        #[test_case(one, with(1, 1))]
        fn inner(val: usize, other: usize) {
            assert_eq!(val, other);
        }

        inner_zero();
        inner_one();
    }

    #[test]
    fn parameterized_argwith_multiple_types_and_args_and_cases() {
        #[test_case(one_plus_one, with(1, 1, 2))]
        #[test_case(one_plus_two, with(1, 2, 3))]
        fn inner(first: isize, second: usize, third: u8) {
            assert_eq!((first + second as isize) as u8, third);
        }

        inner_one_plus_one();
        inner_one_plus_two();
    }

    #[test]
    fn parameterized_argwith_ducked_types() {
        #[test_case(with(0, 1))]
        fn inner(first: _, second: _) {
            assert_eq!(first, 0usize);
            assert_eq!(second, 0usize);
            assert_eq!(first + second, 1usize);
        }

        inner();
    }

    #[test]
    fn parameterized_argwith_multiple_user_defined_types() {
        #[derive(Debug, PartialEq)]
        struct MyStruct(usize);
        
        #[derive(Debug, PartialEq)]
        enum MyEnum {
            One(usize),
            Two(usize)
        }

        #[test_case(struct, with(MyStruct(0), MyStruct(1)))]
        #[test_case(enum, with(MyEnum::One(0), MyEnum::Two(0)))]
        fn inner(first: _, second: _) {
            assert_ne!(first, second);
        }

        inner_struct();
        inner_enum();
    }
}
mod multiple_args {
    use super::test_case;

    #[test]
    fn parameterized_test_case_argwith_argname() {
        #[test_case(one, with(0))]
        fn inner(val: usize) {
            assert_eq!(val, 0usize);
        }

        #[test_case(with(0), two)]
        fn inner(val: usize) {
            assert_eq!(val, 0usize);
        }

        inner_one();
        inner_two();
    }
}
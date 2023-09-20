use sith::test_case;

#[test_case]
fn unparameterized() {
    println!("Hello from a unparameterized test!");
}

#[test_case(with(123))]
fn parameterized(my_arg: usize) {
    println!("my_arg is: {}!", my_arg);
}

#[test_case(one, with(1))]
#[test_case(two, with(2))]
fn parameterized_double(my_arg: usize) {
    println!("my_arg is: {}!", my_arg);
}

#[test_case(one, with(1))]
#[test_case(two, with((1, 2)))]
fn parameterized_sith_magic(my_arg: _) {
    println!("my_arg is: {:?}!", my_arg);
}

#[test_case(format_one, with(verbatim("format_one {:?}"), verbatim(usize)))]
#[test_case(format_two, with(verbatim("format_two {:?}"), verbatim(f32)))]
fn extreme_sith_magic(r#formatter: _, r#my_arg: _) {
    println!(r#formatter, <r#my_arg>::default());
}

fn main() {}
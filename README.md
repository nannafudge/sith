# Sith

Minimal-dependency test harness framework via. simple (definitely unhygenic) AST injection, featuring:

- **Parameterized tests** via: `#[test_case]`
- **Common Setup/Teardown routines** via: `#[test_suite]`

*and potentially more to come...*

![](https://i.imgur.com/WaQV8df.gif)

## Join the dark side - test like a Sith!

***(Note - with great (or well, unlimited) power comes great responsibility)***

---

### Note

**Sith was built specifically for another project of mine - features are added as I need currently. Feel free to drop by with [issues](https://github.com/nannafudge/sith/issues) and [feature requests](https://github.com/nannafudge/sith/labels/enhancement) if you like the concept or are finding this useful!**

---

### Getting Started

To begin weilding ultimate power, simply import `sith` like so:

```rust
use sith::test_case;
use sith::test_suite;
```

> **NOTE**: *Sith is very immature - therefore has no deployment on crates et al. If you'd like to try it out, feel free to clone - but be aware it's still a WIP and lacks testing some features*

---

### Defining test cases with `#[test_case]`

Unparameterized, `#[test_case]` behaves exactly as rust's built-in `#[test]`. Sith  supports both `#[test]` *and* `#[test_case]` - **feel free to use either.**

```rust
#[test_case]
fn simple_test() {
    println!("Hello, World!");
}
```
Outputs:
```
running 1 test
Hello, World!
test simple_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Parameterized Tests with `#[test_case]`

Sith also supports parameterization of `#[test_case]` via ***`arguments`***. Currently, the following arguments are provided:

### `#[test_case(`**`name...`**`)]`:

***Appends **`name...`** to the test function definition***

```rust
#[test_case(one)]
#[test_case(two)]
fn simple_test() {
    println!("Hello, World!");
}
```

Outputs:

```
running 2 tests
Hello, World!
Hello, World!
test simple_test_one ... ok
test simple_test_two ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### `#[test_case(`**`with`**`(...))]`:

***Positionally binds a test input to a value***

Using `with()`, one can pass in instantiated types on a test-by-test basis, allowing different tests to be ran with different inputs. `with()` currently supports two input forms or ***`subarguments`***:

- **`with(...)`** - Values unrecognized as sub-arguments (non-keywords) are by default interpreted as an instance of a `type` and passed in as such **(default)**
- **`with(vertabim(...))`**  - Values within a `verbatim()` sub-argument are *uninterpreted*: Tokens within are output to the syntax tree **without parsing**. This allows passing in of **arbitrary input**, and thus, arbitrary parameterization of tests

#### Simple value-binding [`with(...)`]

```rust
#[test_case(one, with("one!"))]
#[test_case(two, with("two!"))]
fn simple_test(value: &str) {
    println!("Hello from {}", value);
}
```

Outputs:

```
running 2 tests
Hello from one!
Hello from two!
test simple_test_one ... ok
test simple_test_two ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

##### NOTE: Positional Binding

Binding of inputs to function args is done in order of their definition, so make sure that checks out!

```rust
#[test_case(that_will_break_again, with("one is:", 1))]
fn simple_test(formatter: &usize, value: &str) {
    println!("{} {}", formatter, value);
}
```

Outputs:

```
error[E0308]: mismatched types
 --> examples/simple.rs:3:52
  |
3 | #[test_case(that_will_break_again, with("one is:", 1))]
  |                                                    ^ expected `&str`, found integer
4 | fn simple_test(formatter: &usize, value: &str) {
  |                                          ---- expected due to this

error[E0308]: mismatched types
 --> examples/simple.rs:3:41
  |
3 | #[test_case(that_will_break_again, with("one is:", 1))]
  |                                         ^^^^^^^^^ expected `&usize`, found `&str`
4 | fn simple_test(formatter: &usize, value: &str) {
  |                           ------ expected due to this
  |
  = note: expected reference `&usize`
             found reference `&'static str`
```

##### Ducking Types

Binding is *type-sensitive* - that is, the annotated type on the corresponding test function arg **must** match that of the value in `with()`:

```rust
// Attempting to pass in &str as &usize...
#[test_case(that_will_break, with("one!"))]
fn simple_test(value: &usize) {
    println!("Hello from {}", value);
}
```

Outputs:

```
error[E0308]: mismatched types
 --> examples/simple.rs:3:35
  |
3 | #[test_case(that_will_break, with("one!"))]
  |                                   ^^^^^^ expected `&usize`, found `&str`
4 | fn simple_test(value: &usize) {
  |                       ------ expected due to this
  |
  = note: expected reference `&usize`
             found reference `&'static str`
```

Alternatively, one may duck their input using the built-in infer type: `_`

```rust
#[test_case(that_i_fixed, with("one!"))]
fn simple_test(value: &_) {
    println!("Hello from {}", value);
}
```

Outputs:

```
running 1 test
Hello from one!
test simple_test_that_i_fixed ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

#### Arbitrary value-binding [`with(vertabim(...))`]

Using `vertabim()`, one may (truly) harness **unlimited power**:

```rust
#[test_case(float, with(verbatim(f64), 0.0))]
#[test_case(uint, with(verbatim(u64), 0))]
fn default(r#type: _, expected: _) {
    assert_eq!(<r#type>::default(), expected);
}
```

Outputs:

```
running 2 tests
test default_float ... ok
test default_uint ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

### Defining test suites with `#[test_suite]`

As is traditional - `#[test_suite]` describes a collection of tests that're common with one another. Additional to grouping, `#[test_suite]` allows one to define _**common** setup and **teardown**_ routines, ran before and after each test respectively.

> NOTE: Defined as a module, `#[test_suite]` acts literally like a module. Therefore, any dependencies will need to be imported with `use` inside the `#[test_suite]`. This includes `sith::test_case`. Of course, being a module, `super` also works.

Unparameterized, `#[test_suite]` is fairly useless:

```rust
#[test_suite]
mod my_suite {
    use sith::test_case;

    #[test_case]
    fn simple_test() {
        println!("Hello, World!");
    }
}
```

Outputs:

```
running 1 test
Hello, World!
test my_suite::simple_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### `#[setup]`:

***Executes contained code `before` every test***

Using `#[setup]`, one may define common routines that're ran **before** each test in the suite: 

```rust
#[test_suite]
mod my_suite {
    use sith::test_case;

    #[setup]
    fn setup() {
        println!("Setup!");
    }

    #[test_case]
    fn simple_test() {
        println!("Hello, World!");
    }
}
```

Output:

```
running 1 test
Setup!
Hello, World!
test my_suite::simple_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### `#[teardown]`:

***Executes contained code `after` every test***

Using `#[teardown]`, one may define common routines that're ran **after** each test in the suite:

```rust
#[test_suite]
mod my_suite {
    use sith::test_case;

    #[teardown]
    fn teardown() {
        println!("Teardown!");
    }

    #[test_case]
    fn simple_test() {
        println!("Hello, World!");
    }
}
```

Output:

```
running 1 test
Hello, World!
Teardown!
test my_suite::simple_test ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

#### But ***why?***

Well, doing so allows testing different interfaces that utilize the same underlying logic. For example, given a set of binary tree implementations:

```rust
let btreeset = MyBTreeSet::new();
let btreemap = MyBTreeMap::new();
```

Each has a seperate interface for inserting/deleting elements:

```rust
btreeset.insert(1);
btreemap.insert(1, 123);
```

And we wish to test insertion functions correctly **against their particular interfaces themselves**, rather than the underlying. For example, BTreeMap under the hood utilizes BTreeSet to store `OccupiedEntry<'a, K, V, A>` - an abstraction to allow utilization of existing code. We could test `btreemap` like so:

```rust

// BTreeMapGenerator: Returns OccupiedEntry<...> structs
#[test_case(btreeset, with(MyBTreeSet::new(), BTreeSetGenerator::new()))]
#[test_case(btreemap, with(MyBTreeMap::new(), BTreeMapGenerator::new()))]
fn insert_order(btree: _, arg_gen: _) {
    for _ in 0..32 {
        btree.insert(arg_gen.next())
    }

    let iter = btree.iter().peekable();
    while let Some(value) = iter.next() {
        let Option::Some(other) = iter.peek() else {
            break;
        };

        assert!(value < other);
    }
}
```

Unfortunately, this only allows us to test the underlying implementation and not the higher-level interface. If we wanted to test the latter, we'd either have to copy-paste the logic (no bueno), or use declerative macros via. `macro_rules`. Unfortunately, the latter becomes rather unweildy for complex tests or large suites - which is where Sith comes in to play.

#### Isn't this insecure, or a potential attack vector?

**Yes!**. Or well, most likely so. Generally, most Procedural Macros admit this pitfall. But that is what it means to harness the power of the dark side...

Luckily, for most projects, there's a rigorous policing of commit activity and code thereof, although there's always potential for things to slip through the cracks. In lieu of such, if safety is paramount, I reccommend looking at [**Watt**, by the venerable dtolnay](https://github.com/dtolnay/watt).
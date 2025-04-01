use std::mem::MaybeUninit;
use std::{rc::Rc, rc::Weak};

/// Helps building a synchronous Rc cyclic reference, from a closure that can fail.
///
/// Will be replaced by [UniqueRc](https://doc.rust-lang.org/stable/alloc/rc/struct.UniqueRc.html), once it gets stabilized.
pub fn new_cyclic_fallible<T, E, F>(f: F) -> Result<Rc<T>, E>
where
    F: FnOnce(Weak<T>) -> Result<T, E>,
{
    let mut result: Result<(), E> = Ok(());
    let maybe_uninit_rc = Rc::<MaybeUninit<T>>::new_cyclic(|weak_uninit| unsafe {
        let raw = Weak::into_raw(weak_uninit.clone());
        let weak = Weak::<T>::from_raw(raw as *const T);
        match f(weak) {
            Ok(t) => MaybeUninit::<T>::new(t),
            Err(err) => {
                result = Err(err);
                MaybeUninit::<T>::uninit()
            }
        }
    });
    result?;
    let raw = Rc::into_raw(maybe_uninit_rc);
    unsafe { Ok(Rc::from_raw(raw as *const T)) }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[derive(Debug)]
    struct StructA {
        name: &'static str,
        b: Rc<StructB>,
    }

    impl PartialEq for StructA {
        fn eq(&self, other: &Self) -> bool {
            self.name == other.name
        }
    }

    #[derive(Debug)]
    struct StructB {
        a: Weak<StructA>,
    }

    fn new_struct_a(weak: Weak<StructA>) -> StructA {
        StructA {
            name: "StructA",
            b: Rc::new(StructB { a: weak }),
        }
    }

    #[test]
    fn test_new_cyclic_fallible_ok() {
        let result: Result<Rc<StructA>, &str> = new_cyclic_fallible(|weak| Ok(new_struct_a(weak)));
        let struct_a = result.unwrap();
        assert_eq!(struct_a.name, "StructA");
        assert_eq!(struct_a.b.a.upgrade().unwrap().name, "StructA");
    }

    #[test]
    fn test_new_cyclic_fallible_err() {
        let result = new_cyclic_fallible(|weak| {
            let _ = new_struct_a(weak);
            Err("error")
        });
        assert_eq!(result, Err("error"))
    }
}

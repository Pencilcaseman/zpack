use super::consumer::Consumer;

use super::map::Map;
use super::then::{IgnoreThen, Then, ThenIgnore};

pub trait ConsumerExt
where
    Self: Sized + Consumer,
{
    fn then<T>(self, second: T) -> Then<Self, T>
    where
        T: Consumer;

    fn ignore_then<T>(self, second: T) -> IgnoreThen<Self, T>
    where
        T: Consumer;

    fn then_ignore<T>(self, second: T) -> ThenIgnore<Self, T>
    where
        T: Consumer;

    fn map<F, R>(self, function: F) -> Map<Self, F, R>
    where
        F: Fn(<Self as Consumer>::Output) -> R;
}

impl<C> ConsumerExt for C
where
    C: Consumer,
{
    fn then<T>(self, second: T) -> Then<Self, T>
    where
        T: Consumer,
    {
        Then::new(self, second)
    }

    fn ignore_then<T>(self, second: T) -> IgnoreThen<Self, T>
    where
        T: Consumer,
    {
        IgnoreThen::new(self, second)
    }

    fn then_ignore<T>(self, second: T) -> ThenIgnore<Self, T>
    where
        T: Consumer,
    {
        ThenIgnore::new(self, second)
    }

    fn map<F, R>(self, function: F) -> Map<Self, F, R>
    where
        F: Fn(<Self as Consumer>::Output) -> R,
    {
        Map::new(self, function)
    }
}

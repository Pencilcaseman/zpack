use color_eyre::Result;

use super::Consumer;
use super::OptionalConsumer;

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
        F: Fn(<Self as Consumer>::Output) -> Result<R>;

    fn maybe<T>(self, opt: T) -> Then<Self, OptionalConsumer<T>>
    where
        T: Consumer;
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
        F: Fn(<Self as Consumer>::Output) -> Result<R>,
    {
        Map::new(self, function)
    }

    fn maybe<T>(self, opt: T) -> Then<Self, OptionalConsumer<T>>
    where
        T: Consumer,
    {
        Then::new(self, OptionalConsumer::new(opt))
    }
}

//! A crate to provide mutexes which the Rust type system can prove are
//! free from the risk of deadlocks. Inspired by Netstack3 framework.

use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::{Mutex, MutexGuard, PoisonError},
    cell::Cell, // used for thread-local storage (used for OuterMutexPermission)
};

/// A macro to create a unique type for mutex identification.
#[macro_export]
macro_rules! unique_type {
    () => {
        || {}
    };
}

/// A convenience macro to declare mutex identifiers.
#[macro_export]
macro_rules! declare_mutex_identifier {
    ($mutex_name:ident) => {
        struct $mutex_name;
    };
}

/// Some type of permission token required to claim a mutex.
pub trait MutexPermission: 'static {}

impl MutexPermission for OuterMutexPermission {}

/// Permission to claim an "outer" mutex. That is, a class of mutexes where
/// only one can be claimed at once in each thread, thus preventing deadlock.
pub struct OuterMutexPermission(PhantomData<Rc<()>>);

// Note: OuterMutexPermission is designed to be thread-local and not Send
// We'll enforce this through usage patterns rather than negative trait bounds

thread_local! {
    pub static MUTEX_PERMISSION_TOKEN: Cell<Option<OuterMutexPermission>>
        = Cell::new(Some(OuterMutexPermission(PhantomData)));
}

impl OuterMutexPermission {
    /// Get the thread-local mutex claiming permission. This can be called exactly once
    /// per thread, and will panic if it's called more than once in a thread.
    pub fn get() -> OuterMutexPermission {
        MUTEX_PERMISSION_TOKEN
            .with(|token_ref| token_ref.take())
            .expect("Mutex permission already claimed for this thread")
    }
}

/// Permission to claim some nested mutex.
pub struct NestedMutexPermission<P: MutexPermission, I: 'static>(
    PhantomData<Rc<()>>,
    PhantomData<P>,
    PhantomData<I>,
);

impl<P: MutexPermission, I: 'static> MutexPermission for NestedMutexPermission<P, I> {}

/// Permission to claim mutexes in a specific sequence.
pub struct SequentialMutexPermission<P: MutexPermission, I: 'static>(PhantomData<Rc<()>>, P, PhantomData<I>);

impl<P: MutexPermission, I: 'static> SequentialMutexPermission<P, I> {
    fn new(permission: P) -> Self {
        Self(PhantomData, permission, PhantomData)
    }

    /// Consumes this sequential permission to return the permission
    /// token earlier in the sequence.
    pub fn to_earlier(self) -> P {
        self.1
    }
}

impl<P: MutexPermission, I: 'static> MutexPermission for SequentialMutexPermission<P, I> {}

/// Wrapper to make permission types Send/Sync for internal use.
struct PermissionSyncSendWrapper<P: MutexPermission>(P);

/// Safety: These types are only used within PhantomData and not exposed.
unsafe impl<P: MutexPermission> Send for PermissionSyncSendWrapper<P> {}
unsafe impl<P: MutexPermission> Sync for PermissionSyncSendWrapper<P> {}

/// A mutex which is compile-time guaranteed not to deadlock.
/// Similar to the Netstack3 approach for preventing network stack deadlocks.
pub struct DeadlockProofMutex<T, P: MutexPermission, I: 'static>(
    Mutex<T>,
    PhantomData<PermissionSyncSendWrapper<P>>,
    PhantomData<I>,
);

impl<T, P: MutexPermission, I: 'static> DeadlockProofMutex<T, P, I> {
    /// Create a new deadlock-proof mutex.
    pub fn new(content: T, _identifier: I) -> Self {
        Self(Mutex::new(content), PhantomData, PhantomData)
    }

    /// Acquires this mutex, blocking the current thread until it is able to do so.
    pub fn lock(
        &self,
        permission: P,
    ) -> Result<DeadlockProofMutexGuard<T, P, I>, PoisonError<MutexGuard<T>>> {
        self.0
            .lock()
            .map(|guard| DeadlockProofMutexGuard(guard, permission, PhantomData))
    }

    /// Acquires this mutex and provides a token for claiming nested mutexes.
    pub fn lock_for_nested(
        &self,
        permission: P,
    ) -> Result<
        (
            DeadlockProofNestedMutexGuard<T, P, I>,
            NestedMutexPermission<P, I>,
        ),
        PoisonError<MutexGuard<T>>,
    > {
        self.0.lock().map(|guard| {
            (
                DeadlockProofNestedMutexGuard(guard, permission, PhantomData),
                NestedMutexPermission(PhantomData, PhantomData, PhantomData),
            )
        })
    }
}

/// Deadlock-proof equivalent to MutexGuard.
pub struct DeadlockProofMutexGuard<'a, T, P: MutexPermission, I: 'static>(
    MutexGuard<'a, T>,
    P,
    PhantomData<I>,
);

impl<'a, T, P: MutexPermission, I: 'static> DeadlockProofMutexGuard<'a, T, P, I> {
    /// Unlock the mutex and return the permission token.
    pub fn unlock(self) -> P {
        self.1
    }

    /// Unlock the mutex and return a sequential permission token.
    pub fn unlock_for_sequential(self) -> SequentialMutexPermission<P, I> {
        SequentialMutexPermission::new(self.1)
    }
}

impl<T, P: MutexPermission, I: 'static> Deref for DeadlockProofMutexGuard<'_, T, P, I> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.deref()
    }
}

impl<T, P: MutexPermission, I: 'static> DerefMut for DeadlockProofMutexGuard<'_, T, P, I> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.deref_mut()
    }
}

/// Deadlock-proof guard for nested mutex operations.
pub struct DeadlockProofNestedMutexGuard<'a, T, P: MutexPermission, I: 'static>(
    MutexGuard<'a, T>,
    P,
    PhantomData<I>,
);

impl<'a, T, P: MutexPermission, I: 'static> DeadlockProofNestedMutexGuard<'a, T, P, I> {
    /// Unlock the mutex with the nested permission token.
    pub fn unlock(self, _token: NestedMutexPermission<P, I>) -> P {
        self.1
    }

    /// Unlock the mutex and return a sequential permission token.
    pub fn unlock_for_sequential(self) -> SequentialMutexPermission<P, I> {
        SequentialMutexPermission::new(self.1)
    }
}

impl<T, P: MutexPermission, I: 'static> Deref for DeadlockProofNestedMutexGuard<'_, T, P, I> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.deref()
    }
}

impl<T, P: MutexPermission, I: 'static> DerefMut for DeadlockProofNestedMutexGuard<'_, T, P, I> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.deref_mut()
    }
}

// Netstack3-inspired network stack simulation structures
pub struct NetworkStack {
    pub ip_layer: DeadlockProofMutex<IpState, OuterMutexPermission, IpLock>,
    pub device_layer: DeadlockProofMutex<DeviceState, SequentialMutexPermission<OuterMutexPermission, IpLock>, DeviceLock>,
    pub transport_layer: DeadlockProofMutex<TransportState, SequentialMutexPermission<SequentialMutexPermission<OuterMutexPermission, IpLock>, DeviceLock>, TransportLock>,
}

/// Network stack layer states
pub struct IpState {
    pub packets_processed: u64,
    pub routing_table_size: usize,
}

pub struct DeviceState {
    pub interfaces_active: u32,
    pub bytes_transmitted: u64,
}

pub struct TransportState {
    pub tcp_connections: u32,
    pub udp_sockets: u32,
}

/// Lock identifiers for the network stack layers
pub struct IpLock;
pub struct DeviceLock; 
pub struct TransportLock;

impl NetworkStack {
    pub fn new() -> Self {
        Self {
            ip_layer: DeadlockProofMutex::new(
                IpState {
                    packets_processed: 0,
                    routing_table_size: 0,
                },
                IpLock,
            ),
            device_layer: DeadlockProofMutex::new(
                DeviceState {
                    interfaces_active: 0,
                    bytes_transmitted: 0,
                },
                DeviceLock,
            ),
            transport_layer: DeadlockProofMutex::new(
                TransportState {
                    tcp_connections: 0,
                    udp_sockets: 0,
                },
                TransportLock,
            ),
        }
    }
}
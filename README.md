# Deadlock-Mitigation Mutexes using Netstack3

This crate provides a set of mutex wrappers that leverage Rust's type system to guarantee at compile time that deadlocks are impossible. The design is heavily inspired by the locking mechanisms used in Google's Fuchsia OS network stack, Netstack3.

It serves as both a powerful concurrency tool and a practical example of how to encode complex invariants into types, letting the compiler enforce correctness.

## The Problem: Deadlocks in Concurrent Systems

In any multi-threaded system, a deadlock is a critical failure where two or more threads get stuck in a circular wait, each waiting for a resource that the other holds.

Imagine a simplified network stack with two mutexes:

```bash
ip_layer_mutex
```

```bash
device_layer_mutex
```

Now, consider two threads processing packets:

Thread A needs to work from the top down:

Locks ```bash ip_layer_mutex ```

Then, tries to lock ```bash device_layer_mutex```

Thread B needs to work from the bottom up:

Locks ```bash device_layer_mutex ```

Then, tries to lock ```bash ip_layer_mutex ```


Both threads will wait forever. The traditional solution is to enforce a locking order through documentation or convention (e.g., "You must always lock IP before Device"). However, this is prone to human error and can lead to bugs that are difficult to detect and only appear under specific production loads.

## The Netstack3 Solution: Compile-Time Guarantees
Instead of relying on conventions, Netstack3 uses Rust’s powerful type system to make incorrect lock ordering a compile-time error. This crate implements that same philosophy.

The core idea is to use Permission Tokens—special types that act as keys. To acquire a lock on a mutex, a thread must "present" a token of the correct type. The rules for obtaining these tokens enforce a strict hierarchy.

For example:

A thread starts with a single root token: ```OuterMutexPermission```.

To lock the ```ip_layer```, it must consume the ```OuterMutexPermission```.

Unlocking the ```bash ip_layer``` then grants a new token: ```DevicePermission```.

Only this new ```DevicePermission``` token can be used to lock the ``` device_layer```.

You cannot get the ```DevicePermission``` without first having successfully locked and unlocked the```ip_layer```. This sequence is enforced by the compiler, making the deadlock scenario above impossible to write.

## How It Works

Step 1: Understanding Deadlock Conditions (The Coffman Conditions)

For a deadlock to occur, four conditions must be met simultaneously:

1. Mutual Exclusion: Resources (our mutexes) can only be used by one thread at a time. This is fundamental to a mutex's purpose.

2. Hold and Wait: A thread holds at least one resource while waiting to acquire another resource held by a different thread.

3. No Preemption: A resource cannot be forcibly taken away from the thread holding it. It must be released voluntarily.

4. Circular Wait: A set of waiting threads {T₀, T₁, ..., Tₙ} exists such that T₀ is waiting for a resource held by T₁, T₁ is waiting for T₂, ..., and Tₙ is waiting for T₀.

This crate prevents deadlocks by breaking the Circular Wait condition. It achieves this by enforcing a strict, global lock order that makes a circular dependency impossible.


Step 2: Key Concepts in the Implementation

```PhantomData```: The Compiler's Ghost
At the heart of this library is ```std::marker::PhantomData```.

What it is:``` PhantomData<T>``` is a zero-sized marker that tells the Rust compiler that a struct acts as if it contains a value of type ```T```, even though it doesn't at runtime. It has no memory overhead.

Why it's used here: We use ```PhantomData<P>``` inside our ```DeadlockProofMutex``` to associate it with a specific permission token type ```P```. This means we can write a function like ```lock(&self, permission: P)``` where the compiler checks that the ```permission``` argument has the exact type ```P``` that the mutex was defined with. This creates a type-safe link between a lock and its key without any runtime cost.

### Permission Tokens: The Keys to the Locks
The locking rules are encoded in a system of unique permission types.

```OuterMutexPermission```: The root token. Each thread can get exactly one of these when it starts, using a ```thread_local```! variable. This is the entry point to any sequence of locks.

```NestedMutexPermission<P, I>```: A token that grants access to a nested resource. You can only obtain this token by calling ```.lock_for_nested()``` on a mutex, which consumes a parent permission ```P``` and returns this new, more specific permission.

```SequentialMutexPermission<P, I>```: A token that grants access to the next resource in a sequence. You obtain it by calling ```.unlock_for_sequential()``` on a mutex guard, which proves you have finished with and released the previous resource.

### The Type System as the Ultimate Guard
The entire system relies on the Rust compiler's strict type checking and ownership model.

Ownership: When you call ```lock(permission)```, the ```permission``` token is moved (consumed). You no longer own it. You only get it back by calling ```.unlock()``` on the resulting guard. This prevents you from using the same token to lock two different mutexes at the same time.

Generics and Traits: By defining ```DeadlockProofMutex<T, P: MutexPermission, I>```, we create a generic type where ```P``` is the only permission type that will satisfy the compiler for the ```lock``` method. This creates the rigid link between a specific lock and its specific key.

## Instalation

Instructions on how to get a copy of the project and run it on your local machine.

### Prerequisites

_A guide on how to install the tools needed for running the project._

Explain the process step by step.

```bash
git clone https://github.com/Ayushjhax/Deadlock-Mitigation-Netstack3.git 
```
```
cd Deadlock_Prevention
```

```
cargo new
```

# Architecture
 ![- Deadlock](https://github.com/user-attachments/assets/6e2e2b30-8378-4544-be0f-848d457a4f53)
![- Deadlock 2](https://github.com/user-attachments/assets/26e03fad-6eca-4540-b5f8-762b852b1d28)



## Acknowledgments

* [Joshua Liebow-Feeser](https://github.com/joshlf)

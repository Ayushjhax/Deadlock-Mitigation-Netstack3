use std::io::{self, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

mod lib;
use lib::{
    DeadlockProofMutex, NetworkStack, OuterMutexPermission
};

fn main() {
    println!(" Deadlock Prevention System Demo");
    println!("==================================");
    println!("This demo shows compile-time deadlock prevention using Rust's type system.");
    println!("Based on the Netstack3 framework approach.\n");

    loop {
        print_menu();
        let choice = get_user_input("Enter your choice (1-5): ");
        
        match choice.trim() {
            "1" => demo_exclusive_mutexes(),
            "2" => demo_nested_mutexes(),
            "3" => demo_sequential_mutexes(),
            "4" => demo_network_stack(),
            "5" => {
                println!(" Goodbye!");
                break;
            }
            _ => println!(" Invalid choice. Please try again.\n"),
        }
    }
}

fn print_menu() {
    println!("Choose a demo:");
    println!("1. Exclusive Mutexes (One mutex per thread)");
    println!("2. Nested Mutexes (Ordered acquisition)");
    println!("3. Sequential Mutexes (Lock-unlock-lock pattern)");
    println!("4. Network Stack Simulation (Netstack3-style)");
    println!("5. Exit");
}

fn get_user_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input
}

fn demo_exclusive_mutexes() {
    println!("\n Exclusive Mutexes Demo");
    println!("========================");
    println!("Each thread can only hold one mutex at a time, preventing deadlock.");
    
    let mutex1 = Arc::new(DeadlockProofMutex::new(0i32, unique_type!()));
    let mutex2 = Arc::new(DeadlockProofMutex::new(0i32, unique_type!()));
    
    // Clone for the spawned thread
    let c_mutex1 = Arc::clone(&mutex1);
    let c_mutex2 = Arc::clone(&mutex2);
    
    println!(" Spawning thread to modify mutexes...");
    
    let handle = thread::spawn(move || {
        let permission = OuterMutexPermission::get();
        
        println!("  Thread: Acquiring mutex1...");
        let mut guard1 = c_mutex1.lock(permission).unwrap();
        *guard1 = 42;
        println!("  Thread: Set mutex1 to {}", *guard1);
        
        let permission = guard1.unlock();
        println!("  Thread: Released mutex1, acquiring mutex2...");
        
        let mut guard2 = c_mutex2.lock(permission).unwrap();
        *guard2 = 84;
        println!("  Thread: Set mutex2 to {}", *guard2);
    });
    
    handle.join().unwrap();
    
    // Main thread access
    let permission = OuterMutexPermission::get();
    let guard1 = mutex1.lock(permission).unwrap();
    println!("Main: mutex1 = {}", *guard1);
    
    let permission = guard1.unlock();
    let guard2 = mutex2.lock(permission).unwrap();
    println!("Main: mutex2 = {}", *guard2);
    
    println!(" Demo completed successfully!\n");
}

fn demo_nested_mutexes() {
    println!("\n Nested Mutexes Demo");
    println!("======================");
    println!("Mutexes must be acquired in a specific nested order across all threads.");
    
    let mutex1 = Arc::new(DeadlockProofMutex::new(String::from("Layer 1"), unique_type!()));
    let mutex2 = Arc::new(DeadlockProofMutex::new(String::from("Layer 2"), unique_type!()));
    let mutex3 = Arc::new(DeadlockProofMutex::new(String::from("Layer 3"), unique_type!()));
    
    let c_mutex1 = Arc::clone(&mutex1);
    let c_mutex2 = Arc::clone(&mutex2);
    let c_mutex3 = Arc::clone(&mutex3);
    
    println!(" Spawning thread with nested locking...");
    
    let handle = thread::spawn(move || {
        let permission = OuterMutexPermission::get();
        
        println!("  Thread: Acquiring outermost mutex...");
        let (mut guard1, perm1) = c_mutex1.lock_for_nested(permission).unwrap();
        guard1.push_str(" - Modified by thread");
        println!("  Thread: Modified layer 1: {}", *guard1);
        
        println!("  Thread: Acquiring middle mutex...");
        let (mut guard2, perm2) = c_mutex2.lock_for_nested(perm1).unwrap();
        guard2.push_str(" - Modified by thread");
        println!("  Thread: Modified layer 2: {}", *guard2);
        
        println!("  Thread: Acquiring innermost mutex...");
        let mut guard3 = c_mutex3.lock(perm2).unwrap();
        guard3.push_str(" - Modified by thread");
        println!("  Thread: Modified layer 3: {}", *guard3);
        
        // Unlock in reverse order
        let perm2 = guard3.unlock();
        let perm1 = guard2.unlock(perm2);
        guard1.unlock(perm1);
        println!("  Thread: All mutexes released");
    });
    
    handle.join().unwrap();
    
    // Main thread must follow the same order
    let permission = OuterMutexPermission::get();
    let (guard1, perm1) = mutex1.lock_for_nested(permission).unwrap();
    println!("Main: Layer 1 = {}", *guard1);
    
    let (guard2, perm2) = mutex2.lock_for_nested(perm1).unwrap();
    println!("Main: Layer 2 = {}", *guard2);
    
    let guard3 = mutex3.lock(perm2).unwrap();
    println!("Main: Layer 3 = {}", *guard3);
    
    println!(" Demo completed successfully!\n");
}

fn demo_sequential_mutexes() {
    println!("\n Sequential Mutexes Demo");
    println!("==========================");
    println!("Mutexes are acquired and released in a specific sequence.");
    
    let data1 = Arc::new(DeadlockProofMutex::new(vec![1, 2, 3], unique_type!()));
    let data2 = Arc::new(DeadlockProofMutex::new(vec![4, 5, 6], unique_type!()));
    let data3 = Arc::new(DeadlockProofMutex::new(vec![7, 8, 9], unique_type!()));
    
    let c_data1 = Arc::clone(&data1);
    let c_data2 = Arc::clone(&data2);
    let c_data3 = Arc::clone(&data3);
    
    println!(" Spawning thread with sequential processing...");
    
    let handle = thread::spawn(move || {
        let permission = OuterMutexPermission::get();
        
        // Process data1
        println!("  Thread: Processing data set 1...");
        let mut guard1 = c_data1.lock(permission).unwrap();
        guard1.push(10);
        println!("  Thread: Added 10 to data1: {:?}", *guard1);
        let perm = guard1.unlock_for_sequential();
        
        // Process data2
        println!("  Thread: Processing data set 2...");
        let mut guard2 = c_data2.lock(perm).unwrap();
        guard2.push(11);
        println!("  Thread: Added 11 to data2: {:?}", *guard2);
        let perm = guard2.unlock_for_sequential();
        
        // Process data3
        println!("  Thread: Processing data set 3...");
        let mut guard3 = c_data3.lock(perm).unwrap();
        guard3.push(12);
        println!("  Thread: Added 12 to data3: {:?}", *guard3);
        
        println!("  Thread: Sequential processing complete");
    });
    
    handle.join().unwrap();
    
    // Main thread follows same sequence
    let permission = OuterMutexPermission::get();
    
    let guard1 = data1.lock(permission).unwrap();
    println!("Main: Data1 final state: {:?}", *guard1);
    let perm = guard1.unlock_for_sequential();
    
    let guard2 = data2.lock(perm).unwrap();
    println!("Main: Data2 final state: {:?}", *guard2);
    let perm = guard2.unlock_for_sequential();
    
    let guard3 = data3.lock(perm).unwrap();
    println!("Main: Data3 final state: {:?}", *guard3);
    
    println!(" Demo completed successfully!\n");
}

fn demo_network_stack() {
    println!("\n Network Stack Simulation (Netstack3-inspired)");
    println!("=================================================");
    println!("Simulating a network stack with layered mutex acquisition.");
    
    let stack = Arc::new(NetworkStack::new());
    let c_stack = Arc::clone(&stack);
    
    println!(" Spawning network processing thread...");
    
    let handle = thread::spawn(move || {
        let permission = OuterMutexPermission::get();
        
        // Process in network stack order: IP -> Device -> Transport
        println!("  Thread: Processing IP layer...");
        let mut ip_guard = c_stack.ip_layer.lock(permission).unwrap();
        ip_guard.packets_processed += 100;
        ip_guard.routing_table_size = 50;
        println!("  Thread: IP layer - packets: {}, routing entries: {}", 
                ip_guard.packets_processed, ip_guard.routing_table_size);
        
        let device_perm = ip_guard.unlock_for_sequential();
        
        println!("  Thread: Processing Device layer...");
        let mut device_guard = c_stack.device_layer.lock(device_perm).unwrap();
        device_guard.interfaces_active = 3;
        device_guard.bytes_transmitted += 1024;
        println!("  Thread: Device layer - interfaces: {}, bytes: {}", 
                device_guard.interfaces_active, device_guard.bytes_transmitted);
        
        let transport_perm = device_guard.unlock_for_sequential();
        
        println!("  Thread: Processing Transport layer...");
        let mut transport_guard = c_stack.transport_layer.lock(transport_perm).unwrap();
        transport_guard.tcp_connections = 5;
        transport_guard.udp_sockets = 8;
        println!("  Thread: Transport layer - TCP: {}, UDP: {}", 
                transport_guard.tcp_connections, transport_guard.udp_sockets);
        
        println!("  Thread: Network stack processing complete");
        
        // Simulate some processing time
        thread::sleep(Duration::from_millis(100));
    });
    
    handle.join().unwrap();
    
    // Main thread reads the final state
    println!(" Reading final network stack state...");
    let permission = OuterMutexPermission::get();
    
    let ip_guard = stack.ip_layer.lock(permission).unwrap();
    println!("Main: IP Layer - Packets processed: {}, Routing table size: {}", 
            ip_guard.packets_processed, ip_guard.routing_table_size);
    let device_perm = ip_guard.unlock_for_sequential();
    
    let device_guard = stack.device_layer.lock(device_perm).unwrap();
    println!("Main: Device Layer - Active interfaces: {}, Bytes transmitted: {}", 
            device_guard.interfaces_active, device_guard.bytes_transmitted);
    let transport_perm = device_guard.unlock_for_sequential();
    
    let transport_guard = stack.transport_layer.lock(transport_perm).unwrap();
    println!("Main: Transport Layer - TCP connections: {}, UDP sockets: {}", 
            transport_guard.tcp_connections, transport_guard.udp_sockets);
    
    println!(" Network stack simulation completed successfully!\n");
}
A channel for single updatable values. The `Updater` can update a shared value and 
a `Receiver` can then update it's own internal value. 

Example: 

```rust
    use update_channel::channel_with;
    use std::thread::spawn;

    let (mut receiver, updater) = channel_with(0);
    assert_eq!(*receiver.borrow(), 0);

    spawn(move || {
        updater.update(2).unwrap(); // shared value is 2
        updater.update(12).unwrap(); // shared value is 12
    })
    .join().unwrap();

    // Shared value is 2 but internal value is 0
    assert_eq!(*receiver.borrow(), 0);
    // Update the latest value
    receiver.recv_update().unwrap();
    // Shared value is 12 and internal value 12
    assert_eq!(*receiver.borrow(), 12);
```

update_channel is distributed under the terms of the MIT license.

See LICENSE for more details

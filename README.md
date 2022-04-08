# Simple payment engine
A simple payment engine as a Rust project

## V1
Explanation:

Testing:
- Manual testing with some test files
- Unit-test covering most of the logic (because of automock limitations some lines couldn't be covered, but with more time that can be fixed as well)

Concerns:
- Funds and transactions are stored separately. What this means is that they can get out of sync if some issue happens between their updates.
- Currently keeping the transactions history in memory, which will limit in case the number of distinct transactions reaches the maximum possible (u32::MAX). To overcome that we can use some database engine to keep the history in the storage. I recently found the `sled` which can be interesting solution to this problem, but we'll need to deal with serialization/deserialization every time. More sophisticated solutions can be built with caching, to minimise the latency impact, but I think it will be bigger than the scope of the project.

Vision:
- I think we should have 1 immutable chain of events for each customer. This chain then 
will contain any change that happen with the account.
- Then this can be stored persistently and create some in-memory data structures when loading. In production the in-memory data structures might not work the best way, so some cache with key-value storage might work better for accessing old transactions for example.
- Another important component is that we should be able to lock one customer details only and allow other customer accounts/transactions to happen in parallel.
- I also see that we can gain quite strong guarantees when applying the transactions even without the relational databases transactions guarantees. I visualize it like this:
![](./Simple%20Payment%20Engine%20V2%20visualisation.jpg)
    - We will have separate chain of events for each customer. These events represent any change of state for the customer, including transactions, disputes, etc..
    - When we are processing a new event, we lock the chain, add record to it, which has status=`Started`. Then we start doing any other activities that should always be in sync with the customer state, such as updating the transcations storage. When updating the transactions storage, we keep the last *approved* state for that transaction and the *proposed* new one with the event ID that contains the change. Then after finishing all of this, we come back and update the status of the event to `Done`. When next time checking the transactions table, we can check if the event was committed or not. If for some reason the event doens't get committed, then neither the account state nor the transaction get approved.
    - This might require some deeper thinking, but might solve the problem with inconsistencies if we start including network calls and database writes in the process.
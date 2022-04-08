# Simple payment engine
A simple payment engine as a Rust project

## V1
Explanation:
- Separating the problem into couple of traits:
    - Transactions history provider
    - Customer account provider
    - Transaction requests reader
- Then I implement each of them separately. 
- Currently using in-memory implementations for the sake of simplicity, but I also started working on database approach (using sled for now), although that will require more time.
- For the transactions history provider, currently it's keeping transactions and their states in separate hashmaps. This way the transaction itself will never change, while its state can change.
- Adding unit-tests where possible, but there are some technical limitations that will require more time to overcome (like the automock limitation of usage with references [link](https://docs.rs/mockall/0.8.3/mockall/#:~:text=Mocking%20generic%20structs%20and%20generic%20traits%20is%20not%20a%20problem.%20The%20mock%20struct%20will%20be%20generic%2C%20too.%20The%20same%20restrictions%20apply%20as%20with%20mocking%20generic%20methods%3A%20each%20generic%20parameter%20must%20be%20%27static%2C%20and%20generic%20lifetime%20parameters%20are%20not%20allowed.))
- Allowing the available funds to go to negative, as I think this protects the customers from possibly malicious vendors.
- Enforcing the decimal precision when noticing anomalies in the source data.
- When account is frozen/locked, we are not allowing the client to withdraw money, but still letting them to deposit/dispute/resolve/chargeback, because this again can protect the customers from malicious vendors. Imagine one vendor taking multiple incorrect deposits, then the first one gets charged back, we don't want to block the other customers from receiving their money back.
- Not letting to dispute already disputed or charged back transaction.
- Not letting to chargeback/resolve non-disputed transactions.

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
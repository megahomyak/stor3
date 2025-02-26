pub struct Stor3 {
    db: std::fs::File,
    log: std::fs::File,
}

/*

* list(prefix) -> vec<continuation> # breadth first
* remove(key) -> bool (is removed)
* add(key) -> nil
* check(key) -> bool (is found)
* commit() -> nil # flush the log into the db?

since it's a tree, here's how it would be described in C:
struct Node {
    left: *Node, // nullable
    right: *Node, // nullable
}
and then the root node should probably be a pointer, not a full node, but then a check for "" will have to always yield "true", but that maybe is not the best thing to check for because it's basically checking for "anything"

what I need for alloc and dealloc: quick way to get a new node, quick way to make a previously-taken node available for new taking

Essentially, just a linked list of empty nodes would already be good enough. Something like:
* Empty node: position of next empty node
* When a new node becomes empty, just add it to the head of the list
* When taking a new node, grab one from the list, or if the list is empty, then just take from the end of the file. In the grabbed node, get the address, put it into the global "next node" place, and then populate the newly-received node and use it as normal

layout: header + vacant node pointer + root filled node pointer

allocate_node() -> node_address:
    if (*vacant_node_pointer_pointer == NULL) {
        return eof
    } else {
        return swap(*vacant_node_pointer_pointer, **vacant_node_pointer_pointer)
        // OLD STUFF:
        // old_vacant_node_pointer = *vacant_node_pointer_pointer
        // *vacant_node_pointer = *old_vacant_node_pointer
        // return old_vacant_node_pointer
    }

deallocate_node(node_address):
    *node_address = *vacant_node_pointer_pointer # Harmless, because the node will be destroyed anyway
    *vacant_node_pointer_pointer = node_address

GREATEST CONCERN AT THE MOMENT: making it resilient to sudden power loss, SO: none of the files should be left in a broken state at any moment. *WHAT'S A "BROKEN STATE"*: a copied pointer (some data area can be retrieved by two separate pointers, which should technically never happen ever with the current structure), some node that's not pointed at (so, a memory leak because the node will never be freed). THAT MEANS:
* When an area is allocated, a pointer to it needs to be stored immediately
* When swapping pointers, there should be some safety mechanism to prevent situations such as AB=>AA

Pointer swap routine:
* NEVER USE AN IN-MEMORY TEMP FOR SWAPPING TO HANDLE POWER LOSS
* Three slots in file: ptr1, ptr2, temp. Pointer swap that can be interrupted:
    * we start with <NON-PTR VALUE> in temp
    * if (temp == <NON-PTR VALUE>) cp ptr1 to temp
    * cp ptr2 to ptr1

- regular swapping:
* cp ptr1 to temp
* cp ptr2 to ptr1
* cp temp to ptr2

A B T
0 1 x # init
0 1 0 # cp ptr1 to temp
1 1 0 # cp ptr2 to ptr1
1 0 0 # cp temp to ptr2

INVARIANTS:
* We know the values swapped CAN'T BE THE SAME - actually, no matter how the operation ends, if it won't put an <UNINIT> in one of the values, the result should be the same, so I don't think uniqueness is something I should care about at all
* We know no value can collide with the <UNINIT> value (marked as "x")

A B T
? ? ? # init

How do we know what steps have passed?:
* "cp ptr1 to temp": temp is not <UNINIT>
* "cp ptr2 to ptr1": A == B
* "cp temp to ptr2": we don't actually have to check this one, we can just do this procedure anyway

procedure of addition: we just go through every node (r/o) and if we see a node missing, we add it (after that, the operation can be interrupted, and the next pass will just go over the added node)

So, essentially:
For "A -> B", inserting "A -> B -> C":
    root -> A
    A -> B
    B -> none seen!
    WRITING {
        alloc a node;
        put where "C" is supposed to be;
        fill the node;
    }

ASSUMING THE DATABASE IS NON-CONCURRENT atm:
swap(): given two values, can be interrupted. Yet if redone, will return to the old state, which is wrong
alloc() <- swap(): can be interrupted
"C" is either "0" or "1", so just filling the current node's slot with the received pointer - we might be screwed if there's an allocation that's not tracked anywhere. So one way to circumvent that would be to write the pointer to the allocation first, but then we don't know if it's initialised
...

If I split the entire process into fallible operations and keep track what OP I'm currently on, I will be able to isolate each operation and thus be sure that the operations stayed in order

In reality, if the only atomicity I have is writing just one bit, then I have to get creative for bigger atomicity (such as writing whole pointers: it'll probably make sense to have a buffer of what bits were written)

I feel like I'm missing something and this whole idea is worthless and won't be doable with the tools that UNIX gives me
*/

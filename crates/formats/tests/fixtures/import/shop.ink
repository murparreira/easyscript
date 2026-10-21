VAR gold = 0
// Where it starts.
-> start

=== start ===
Bob: Hello there! #happy
* [Say hi] Hi, Bob.
    Bob: Nice to see you.
    ~ gold = gold + 5
* {gold > 10} Ask for money
    Bob: No way.
    -> shop
+ (leave) Leave -> END
- Bob: Anyway, you have {gold} coins, {gold > 5: rich|poor}.
{ gold > 3:
    Bob: Rich!
- else:
    Bob: Poor.
}
{ gold:
- 0: Bob: Broke.
- else: Bob: Not broke.
}
-> shop

=== shop ===
= counter
Shopkeeper: What do you want? {~Hi|Hello}.
{
    - gold > 100: Shopkeeper: Big spender.
    - gold > 10 && gold < 50: Shopkeeper: Welcome.
    - else: Shopkeeper: Out.
}
~ gold++
~ buy("apple")
-> visit ->
-> DONE

= visit
Shopkeeper: Look around <> please.
<- chatter
->->

=== function double(x) ===
~ return x * 2

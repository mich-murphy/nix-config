---
name: interview
description: >
  Interview the user relentlessly about a plan or design until reaching shared understanding,
  resolving each branch of the decision tree. Use when user wants to stress-test a plan, get
  thorough review on their design, or mentions "interview".
---

# Interview

Interview me relentlessly about every aspect of plan until
shared understanding reached. Use the host's structured user-input
capability when available. Walk down
each branch of design tree, resolve dependencies between decisions
one by one.

If question can be answered by exploring codebase, explore
codebase instead.

For each question, provide recommended answer.

Once design finalised DO NOT implement. Present succinct and factual
overview of implementation plan which includes:

- class diagram (if code change involving class structures being implemented)
- sequence diagram (if implementing significant architectural change)
- overview of class/function/method interfaces (if code change involving changes
  to classes/methods/functions)

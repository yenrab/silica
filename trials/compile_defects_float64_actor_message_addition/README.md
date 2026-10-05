# compile_defects_float64_actor_message_addition

One valid Silica program whose emitted assembly does not assemble today (see
`defect_float64_actor_message.silica` and the `compile_defects_addition` README for why such a trial gets a
directory of its own: the failed assemble step leaves every other trial in the directory without an executable).

- `defect_float64_actor_message`: a `float64` message cast to a behaviour. The cast prim stages the message
  word from the float let's D register with an integer `MOV`, and the actor dispatch delivers every message
  word in X0 while a float64 first parameter is read from D0. Topic suite: `actors_addition` (next to
  `behaviour_uint32_message_survives_call`).

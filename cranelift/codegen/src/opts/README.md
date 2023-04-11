Rules here are allowed to rewrite pure expressions arbitrarily,
using the same inputs as the original, or fewer. In other words, we
cannot pull a new eclass id out of thin air and refer to it, other
than a piece of the input or a new node that we construct; but we
can freely rewrite e.g. `x+y-y` to `x`.

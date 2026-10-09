/* Le démarrage construit le cœur et l'UI sur la pile interne du cœur 0.
   Refuser un futur assemblage dont les données statiques mangeraient sa marge. */
ASSERT(_stack_start - _stack_end >= 16384, "screen requires at least 16 KiB for the core 0 stack");

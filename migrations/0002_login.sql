-- Anmeldung über Pocket ID für die Listener aus LAGEMONITOR_AUTH_BIND (src/auth.rs).
-- Gespeichert ist nur der SHA-256 des Sitzungsschlüssels; der Schlüssel selbst steht im Cookie.
-- `link_id`: Sitzung aus einem Link ohne Anmeldung (settings `public_links`); sie endet, sobald die
-- Suite den Link nicht mehr schickt.
CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    name TEXT NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    link_id TEXT
);
-- Eine angefangene Anmeldung: State, PKCE-Verifier und wohin es danach geht. Lebt zehn Minuten.
CREATE TABLE logins (
    state TEXT PRIMARY KEY,
    verifier TEXT NOT NULL,
    next TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

-- Verkehrsfluss von TomTom (src/web/traffic.rs): Kacheln mit Stand, damit jede Kachel höchstens alle
-- 30 Minuten bei TomTom abgefragt wird, und die Zahl der Abfragen je Tag (UTC) für die Tagesgrenze.
CREATE TABLE traffic_tiles (
    dark INTEGER NOT NULL,
    z INTEGER NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    fetched_at_ms INTEGER NOT NULL,
    body BLOB NOT NULL,
    PRIMARY KEY (dark, z, x, y)
);
CREATE TABLE traffic_budget (day TEXT PRIMARY KEY, tiles INTEGER NOT NULL);

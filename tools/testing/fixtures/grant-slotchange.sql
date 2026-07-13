-- Entitle the headless partner character to slot moves.
--
-- Hercules gates character slot switching per character via the `slotchange`
-- column (each move decrements it; there is no server config to enable moves
-- globally in this build). The `character-slot-switch` scenario moves the
-- partner character to a free slot and back on every run, so grant a large
-- budget once:
--
--   mysql -u ragnarok -pragnarok ragnarok < tools/testing/fixtures/grant-slotchange.sql
--
-- (Adjust credentials/database to conf/global/sql_connection.conf.)
-- The GM fixture character intentionally keeps slotchange = 0 so the
-- `character-slot-switch-rejected` scenario stays valid.

UPDATE `char` SET `slotchange` = 10000 WHERE `name` = 'HeadlessTwo';

-- Item conservation audit for the disposable integration database.
--
-- Every item movement Hercules performs goes through pc->additem/delitem (or
-- the cart equivalents), which write `picklog` when item logging is on
-- (conf/map/logs.conf `enable: 0xFFFFFFFF`, `log_filter: 1`). So for each
-- character, the sum of its log rows per item must equal what it holds in its
-- inventory and cart. Storage is never logged on its own side: a deposit is a
-- logged inventory removal (type R), a withdrawal a logged addition, so an
-- account's storage must equal minus the sum of its characters' R rows.
--
-- Each row printed is a violation: a duplicated or vanished item, or a
-- movement that bypassed the log. Run against a database whose characters
-- were all created inside it (starting items are not logged; see the
-- `start_items` exclusion below).

-- Starting items handed out by the char-server at creation are not logged.
-- They are listed in conf/char/char-server.conf `start_items`; their holdings
-- may exceed the log by at most the starting amount.
SELECT CONCAT('char ', k.`char_id`, ' item ', k.`nameid`, ': holds ', COALESCE(h.`qty`, 0),
              ', item log totals ', COALESCE(l.`qty`, 0))
  FROM (
        SELECT `char_id`, `nameid` FROM `inventory`
        UNION SELECT `char_id`, `nameid` FROM `cart_inventory`
        UNION SELECT p.`char_id`, p.`nameid` FROM `picklog` p JOIN `char` c ON c.`char_id` = p.`char_id`
       ) k
  LEFT JOIN (
        SELECT `char_id`, `nameid`, SUM(`amount`) AS `qty`
          FROM (SELECT `char_id`, `nameid`, `amount` FROM `inventory`
                UNION ALL SELECT `char_id`, `nameid`, `amount` FROM `cart_inventory`) held
         GROUP BY `char_id`, `nameid`
       ) h ON h.`char_id` = k.`char_id` AND h.`nameid` = k.`nameid`
  LEFT JOIN (
        SELECT p.`char_id`, p.`nameid`, SUM(p.`amount`) AS `qty`
          FROM `picklog` p JOIN `char` c ON c.`char_id` = p.`char_id`
         GROUP BY p.`char_id`, p.`nameid`
       ) l ON l.`char_id` = k.`char_id` AND l.`nameid` = k.`nameid`
 WHERE COALESCE(h.`qty`, 0) <> COALESCE(l.`qty`, 0);

SELECT CONCAT('account ', k.`account_id`, ' storage item ', k.`nameid`, ': holds ', COALESCE(s.`qty`, 0),
              ', deposits minus withdrawals ', COALESCE(r.`qty`, 0))
  FROM (
        SELECT `account_id`, `nameid` FROM `storage`
        UNION SELECT c.`account_id`, p.`nameid` FROM `picklog` p JOIN `char` c ON c.`char_id` = p.`char_id` WHERE p.`type` = 'R'
       ) k
  LEFT JOIN (SELECT `account_id`, `nameid`, SUM(`amount`) AS `qty` FROM `storage` GROUP BY `account_id`, `nameid`) s
         ON s.`account_id` = k.`account_id` AND s.`nameid` = k.`nameid`
  LEFT JOIN (
        SELECT c.`account_id`, p.`nameid`, -SUM(p.`amount`) AS `qty`
          FROM `picklog` p JOIN `char` c ON c.`char_id` = p.`char_id`
         WHERE p.`type` = 'R'
         GROUP BY c.`account_id`, p.`nameid`
       ) r ON r.`account_id` = k.`account_id` AND r.`nameid` = k.`nameid`
 WHERE COALESCE(s.`qty`, 0) <> COALESCE(r.`qty`, 0);

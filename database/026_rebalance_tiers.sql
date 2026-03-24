-- Rebalance rewards tiers to match frontend HTML (Plus=4k, Pro=10k, Elite=30k, Premium=100k)

UPDATE tiers SET min_invest = 0, max_invest = 399999 WHERE name = 'Intro';
UPDATE tiers SET min_invest = 400000, max_invest = 999999 WHERE name = 'Plus';
UPDATE tiers SET min_invest = 1000000, max_invest = 2999999 WHERE name = 'Pro';
UPDATE tiers SET min_invest = 3000000, max_invest = 9999999 WHERE name = 'Elite';
UPDATE tiers SET min_invest = 10000000, max_invest = NULL WHERE name = 'Premium';

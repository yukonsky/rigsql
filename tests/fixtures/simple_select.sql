SELECT
    u.id,
    u.name,
    COUNT(*) AS order_count
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE u.active = TRUE
GROUP BY u.id, u.name
HAVING COUNT(*) > 5
ORDER BY order_count DESC
LIMIT 10;

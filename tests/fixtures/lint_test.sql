select u.id, u.Name, count(*) cnt
from users u
inner join orders o on u.id = o.user_id
where u.active = true
group by u.id, u.Name
having count(*) > 5
order by cnt  desc

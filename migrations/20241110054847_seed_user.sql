-- Add migration script here
INSERT INTO
    users (user_id, username, password_hash)
VALUES
    (
        '27151b40-4d0f-4708-8eb7-54e61c7eea70',
        'admin',
        '$argon2id$v=19$m=19456,t=2,p=1$5fDGx640H2vuA/fl83xeNg$17VrxQy2I/QkYAmZe0bvpAk6tv2l42HI/6rGCEp0+ws'
    )

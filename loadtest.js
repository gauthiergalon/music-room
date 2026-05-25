import http from 'k6/http';
import { check, sleep } from 'k6';

// Load Profile definition
export const options = {
  stages: [
    { duration: '10s', target: 1000 }, // Ramp up to 1000 users
    { duration: '15s', target: 1000 }, // Hold load at 1000 users
    { duration: '5s', target: 0 },     // Ramp down to 0 users
  ],
};

const BASE_URL = 'http://backend:3000';

export default function () {
  // 1. Generate unique data to avoid violating database UNIQUE constraints
  const randomStr = Math.random().toString(36).substring(2, 8);
  const username = `u_${__VU}_${__ITER}_${randomStr}`.substring(0, 24); 
  const email = `${username}@test.com`;

  const registerPayload = JSON.stringify({
    username: username,
    email: email,
    password: "password123",
  });

  const params = { headers: { 'Content-Type': 'application/json' } };

  // 2. User sends a POST request to register
  const regRes = http.post(`${BASE_URL}/auth/register`, registerPayload, params);
  
  check(regRes, {
    'User registered successfully (201)': (r) => r.status === 201,
  });

  // If we received a JWT token, use it to query the database
  let token = null;
  if (regRes.status === 201) {
    token = regRes.json('access_token');
  }

  // 3. With the token, the same user fetches a protected route
  if (token) {
    const authParams = {
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      }
    };

    // Fetch all rooms (requires backend to read from Postgres via ORM)
    const roomsRes = http.get(`${BASE_URL}/rooms`, authParams);
    check(roomsRes, {
      'Rooms listed successfully (200)': (r) => r.status === 200,
    });
  }
}

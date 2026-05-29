# Example .hurl file — replace with real endpoints.
# Run: hurl --test example.hurl
# Or via skill: bash <skill>/scripts/run.sh --axes api

GET {{BASE_URL}}/api/health
HTTP 200
[Asserts]
jsonpath "$.status" == "ok"

# Auth flow — happy path
POST {{BASE_URL}}/api/auth/signup
Content-Type: application/json
{
  "email": "test+{{newUuid}}@example.com",
  "password": "TestPassword1!"
}
HTTP 200
[Captures]
session: cookie "next-auth.session-token"

GET {{BASE_URL}}/api/user/profile
Cookie: next-auth.session-token={{session}}
HTTP 200
[Asserts]
jsonpath "$.email" startsWith "test+"

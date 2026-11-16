`google_test_key.pem` is a throwaway RSA key generated for the test suite only.
The tests sign fake Google ID tokens with it and serve the matching public key
from a mock JWKS endpoint. It protects nothing and is not used anywhere else.

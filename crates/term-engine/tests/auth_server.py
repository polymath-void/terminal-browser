#!/usr/bin/env python3
"""
Real-world, zero-dependency authentication web server with SQLite persistence,
RFC 6265 session cookies, and HTML form endpoints.
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import sqlite3
import hashlib
import urllib.parse
import uuid
import os
import sys
import tempfile

DB_PATH = os.path.join(tempfile.gettempdir(), "real_auth_server.db")

def init_db():
    conn = sqlite3.connect(DB_PATH)
    c = conn.cursor()
    c.execute("""
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)
    c.execute("""
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            username TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    """)
    conn.commit()
    conn.close()

def hash_password(pwd: str) -> str:
    return hashlib.sha256(pwd.encode('utf-8')).hexdigest()

class AuthHandler(BaseHTTPRequestHandler):
    def get_cookie(self, name: str) -> str:
        cookie_hdr = self.headers.get('Cookie')
        if not cookie_hdr:
            return None
        for item in cookie_hdr.split(';'):
            parts = item.strip().split('=', 1)
            if len(parts) == 2 and parts[0] == name:
                return parts[1]
        return None

    def get_current_user(self):
        token = self.get_cookie('auth_session')
        if not token:
            return None
        conn = sqlite3.connect(DB_PATH)
        c = conn.cursor()
        c.execute("SELECT username FROM sessions WHERE token = ?", (token,))
        row = c.fetchone()
        conn.close()
        return row[0] if row else None

    def parse_form_body(self):
        content_len = int(self.headers.get('Content-Length', 0))
        post_body = self.rfile.read(content_len).decode('utf-8')
        return urllib.parse.parse_qs(post_body)

    def do_GET(self):
        path = self.path.split('?')[0]

        if path in ('/', '/login'):
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            html = """<!DOCTYPE html>
<html>
<head><title>Portal Login</title></head>
<body>
    <h1>Portal Login</h1>
    <p>Please enter your credentials to access the secure customer area.</p>
    <form action="/login" method="post">
        <label for="username">Username:</label><br/>
        <input type="text" id="username" name="username" placeholder="Enter username" required /><br/><br/>
        <label for="password">Password:</label><br/>
        <input type="password" id="password" name="password" placeholder="Enter password" required /><br/><br/>
        <button type="submit">Log In</button>
    </form>
    <hr/>
    <p>Need an account? <a href="/signup">Sign up here</a>.</p>
</body>
</html>"""
            self.wfile.write(html.encode('utf-8'))

        elif path == '/signup':
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            html = """<!DOCTYPE html>
<html>
<head><title>Create Account</title></head>
<body>
    <h1>Create New Account</h1>
    <p>Sign up to receive full instant access to your account portal.</p>
    <form action="/signup" method="post">
        <label for="username">Username:</label><br/>
        <input type="text" id="username" name="username" placeholder="Choose username" required /><br/><br/>
        <label for="email">Email Address:</label><br/>
        <input type="text" id="email" name="email" placeholder="name@example.com" required /><br/><br/>
        <label for="password">Password:</label><br/>
        <input type="password" id="password" name="password" placeholder="Choose password" required /><br/><br/>
        <button type="submit">Create Account</button>
    </form>
    <hr/>
    <p>Already have an account? <a href="/login">Login here</a>.</p>
</body>
</html>"""
            self.wfile.write(html.encode('utf-8'))

        elif path == '/dashboard':
            user = self.get_current_user()
            if not user:
                self.send_response(302)
                self.send_header('Location', '/login?error=unauthorized')
                self.end_headers()
                return

            conn = sqlite3.connect(DB_PATH)
            c = conn.cursor()
            c.execute("SELECT id, username, email, created_at FROM users WHERE username = ?", (user,))
            user_data = c.fetchone()
            conn.close()

            if not user_data:
                self.send_response(302)
                self.send_header('Location', '/login')
                self.end_headers()
                return

            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            html = f"""<!DOCTYPE html>
<html>
<head><title>Customer Dashboard</title></head>
<body>
    <h1>Welcome, {user_data[1]}!</h1>
    <p style="color: #00aa00; font-weight: bold;">✔ Authenticated Session Active</p>
    <p>You have successfully logged into the private customer portal.</p>
    
    <h2>Account Profile Summary</h2>
    <table>
        <thead>
            <tr>
                <th>Account ID</th>
                <th>Username</th>
                <th>Email Address</th>
                <th>Account Status</th>
            </tr>
        </thead>
        <tbody>
            <tr>
                <td>#{user_data[0]}</td>
                <td>{user_data[1]}</td>
                <td>{user_data[2]}</td>
                <td>Active &amp; Verified</td>
            </tr>
        </tbody>
    </table>
    
    <hr/>
    <p><a href="/logout">Log Out</a></p>
</body>
</html>"""
            self.wfile.write(html.encode('utf-8'))

        elif path == '/logout':
            token = self.get_cookie('auth_session')
            if token:
                conn = sqlite3.connect(DB_PATH)
                c = conn.cursor()
                c.execute("DELETE FROM sessions WHERE token = ?", (token,))
                conn.commit()
                conn.close()

            self.send_response(302)
            self.send_header('Set-Cookie', 'auth_session=deleted; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT')
            self.send_header('Location', '/login?logged_out=1')
            self.end_headers()

        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        path = self.path.split('?')[0]
        params = self.parse_form_body()

        username = params.get('username', [''])[0].strip()
        password = params.get('password', [''])[0].strip()
        email = params.get('email', [''])[0].strip() or f"{username}@example.com"

        if path == '/signup':
            if not username or not password:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(b"Username and password required")
                return

            conn = sqlite3.connect(DB_PATH)
            c = conn.cursor()
            try:
                pwd_hash = hash_password(password)
                c.execute("INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)",
                          (username, email, pwd_hash))
                
                # Issue new session token
                token = str(uuid.uuid4())
                c.execute("INSERT INTO sessions (token, username) VALUES (?, ?)", (token, username))
                conn.commit()
            except sqlite3.IntegrityError:
                conn.close()
                self.send_response(409)
                self.send_header('Content-Type', 'text/html; charset=utf-8')
                self.end_headers()
                self.wfile.write(b"<h1>User already exists</h1><p><a href='/signup'>Try another</a></p>")
                return
            conn.close()

            self.send_response(302)
            self.send_header('Set-Cookie', f"auth_session={token}; Path=/; HttpOnly; SameSite=Lax")
            self.send_header('Location', '/dashboard')
            self.end_headers()

        elif path == '/login':
            conn = sqlite3.connect(DB_PATH)
            c = conn.cursor()
            pwd_hash = hash_password(password)
            c.execute("SELECT id FROM users WHERE username = ? AND password_hash = ?", (username, pwd_hash))
            row = c.fetchone()

            if not row:
                conn.close()
                self.send_response(401)
                self.send_header('Content-Type', 'text/html; charset=utf-8')
                self.end_headers()
                self.wfile.write(b"<h1>Invalid username or password</h1><p><a href='/login'>Try again</a></p>")
                return

            token = str(uuid.uuid4())
            c.execute("INSERT INTO sessions (token, username) VALUES (?, ?)", (token, username))
            conn.commit()
            conn.close()

            self.send_response(302)
            self.send_header('Set-Cookie', f"auth_session={token}; Path=/; HttpOnly; SameSite=Lax")
            self.send_header('Location', '/dashboard')
            self.end_headers()

        else:
            self.send_response(404)
            self.end_headers()

def main():
    init_db()
    port = 8088
    server = HTTPServer(('127.0.0.1', port), AuthHandler)
    print(f"Auth server listening on http://127.0.0.1:{port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()

if __name__ == '__main__':
    main()

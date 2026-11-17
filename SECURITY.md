# Security policy

strokebind reads raw input devices and injects input events, so security reports are taken seriously.

Report vulnerabilities privately through GitHub's "Report a vulnerability" (Security tab) once the repository is public.
Do not open public issues for security problems. Expect an acknowledgment within a few days.

Design commitments: the daemon never runs as root, never grabs keyboards in 0.x, and releases any device grab on exit, crash or signal.

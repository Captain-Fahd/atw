# Start atw once on tty1 after autologin.
[ -f ~/.bashrc ] && . ~/.bashrc
if [ "$(tty)" = /dev/tty1 ] && [ -z "${ATW_STARTED:-}" ]; then
	export ATW_STARTED=1
	/usr/local/sbin/atw-start
fi

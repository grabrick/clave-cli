#!/usr/bin/env python3
"""Headless-проверка живого рендера clave через эмулятор терминала pyte.

Ручной рендер (src/render.rs) НЕ делает CPR-запрос курсора, поэтому его можно
прогнать под обычным псевдо-TTY и «увидеть» экран. Скрипт запускает настоящий
бинарник, дёргает меню/подсказки и проверяет ключевое поведение «как в Claude
Code»: панель открывается над вводом, история в скроллбэке, без накопления.

Запуск:
    python3 scripts/render_check.py target/release/clave <CLAVE_HOME>
(нужен pyte: `pip3 install pyte`). CLAVE_HOME — каталог с config + chats/.
"""
import fcntl
import os
import pty
import select
import struct
import subprocess
import sys
import termios
import time

import pyte

COLS, ROWS = 100, 30


def main() -> int:
    binary, home = sys.argv[1], sys.argv[2]
    master, slave = pty.openpty()
    fcntl.ioctl(master, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    env = dict(os.environ)
    env.update(CLAVE_HOME=home, CLAVE_SKIP_ONBOARDING="1", TERM="xterm-256color")
    proc = subprocess.Popen(
        [binary], stdin=slave, stdout=slave, stderr=slave, env=env, close_fds=True
    )
    os.close(slave)
    screen = pyte.Screen(COLS, ROWS)
    stream = pyte.ByteStream(screen)

    def pump(seconds=0.4):
        end = time.time() + seconds
        while time.time() < end:
            ready, _, _ = select.select([master], [], [], 0.05)
            if ready:
                try:
                    data = os.read(master, 65536)
                except OSError:
                    return
                if not data:
                    return
                stream.feed(data)

    def send(text):
        os.write(master, text.encode())

    def snap():
        return tuple(row.rstrip() for row in screen.display)

    def visible(needle):
        return any(needle in row for row in screen.display)

    pump(1.2)
    opened = []
    idles = []
    for _ in range(5):
        send("/")
        pump(0.35)
        opened.append(visible("/brainstorming") or visible("/help") or visible("/btw"))
        send("\x7f")  # стереть '/'
        pump(0.35)
        idles.append(snap())
    send("?")
    pump(0.35)
    shortcuts_open = visible("Управление") or visible("Controls")
    send("\x1b")  # Esc
    pump(0.35)
    shortcuts_closed = not (visible("Управление") or visible("Controls"))
    send("/quit\r")
    pump(0.8)

    try:
        os.close(master)
    except OSError:
        pass
    try:
        code = proc.wait(timeout=2)
    except Exception:
        proc.kill()
        code = -1

    # Идл-экраны после каждого закрытия должны совпадать — это и есть «нет
    # накопления/дрейфа» (первый кадр не считаем: он мог быть полноэкранным).
    stable = all(state == idles[0] for state in idles)
    checks = {
        "палитра открывается (5/5)": all(opened),
        "подсказки '?' открываются": shortcuts_open,
        "Esc закрывает подсказки": shortcuts_closed,
        "idle стабилен (нет накопления)": stable,
        "чистый выход /quit": code == 0,
    }
    for name, ok in checks.items():
        print(("  OK   " if ok else "  FAIL ") + name)
    ok = all(checks.values())
    print("\nИТОГ:", "ВСЁ ОК" if ok else "ЕСТЬ ПРОБЛЕМЫ")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

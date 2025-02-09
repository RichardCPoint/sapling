# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2.

# pyre-strict

# TODO(T65013742)

from __future__ import absolute_import, division, print_function, unicode_literals

import ctypes
import errno
import sys

from thrift.transport.TSocket import TSocket
from thrift.transport.TTransport import TTransportException
from typing_extensions import Buffer


if sys.platform == "win32":
    import ctypes.wintypes


# This is a Windows only script which enables the Python Thrift client to use
# Unix Domain Socket. Most of the pyre errors in this files are because of
# missing windll on the linux system.

# WSA Error codes
WSAETIMEDOUT = 10060
WSAECONNREFUSED = 10061
WSAEWOULDBLOCK = 10035
WSATRY_AGAIN = 11002

# Socket options
SO_SNDTIMEO = 0x1005
SO_RCVTIMEO = 0x1006
SOL_SOCKET = 0xFFFF

# ioctlsocket operations
FIONBIO = 2147772030

# int WSAStartup(
#     WORD      wVersionRequired,
#     LPWSADATA lpWSAData
# );
WSADESCRIPTION_LEN: int = 256 + 1
WSASYS_STATUS_LEN: int = 128 + 1


class WSAData64(ctypes.Structure):
    # pyre-fixme[4]: Attribute must be annotated.
    _fields_ = [
        ("wVersion", ctypes.c_ushort),
        ("wHighVersion", ctypes.c_ushort),
        ("iMaxSockets", ctypes.c_ushort),
        ("iMaxUdpDg", ctypes.c_ushort),
        ("lpVendorInfo", ctypes.c_char_p),
        ("szDescription", ctypes.c_ushort * WSADESCRIPTION_LEN),
        ("szSystemStatus", ctypes.c_ushort * WSASYS_STATUS_LEN),
    ]


# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
WSAStartup = ctypes.windll.ws2_32.WSAStartup
WSAStartup.argtypes = [ctypes.wintypes.WORD, ctypes.POINTER(WSAData64)]
WSAStartup.restype = ctypes.c_int


# Win32 socket API
# SOCKET WSAAPI socket(
#   int af,
#   int type,
#   int protocol
# );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
socket = ctypes.windll.ws2_32.socket
socket.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_int]
socket.restype = ctypes.wintypes.HANDLE


# int connect(
#   SOCKET         s,
#   const sockaddr * name,
#   int            namelen
#   );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
connect = ctypes.windll.ws2_32.connect
connect.argtypes = [ctypes.wintypes.HANDLE, ctypes.c_void_p, ctypes.c_int]
connect.restype = ctypes.c_int

# int bind(
#   SOCKET         s,
#   const sockaddr *name,
#   int            namelen
# );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
bind = ctypes.windll.ws2_32.bind
bind.argtypes = [ctypes.wintypes.HANDLE, ctypes.c_void_p, ctypes.c_int]
bind.restype = ctypes.c_int


# int WSAAPI send(
#   SOCKET     s,
#   const char *buf,
#   int        len,
#   int        flags
# );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
send = ctypes.windll.ws2_32.send
send.argtypes = [
    ctypes.wintypes.HANDLE,
    ctypes.POINTER(ctypes.c_char),
    ctypes.c_int,
    ctypes.c_int,
]
send.restype = ctypes.c_int

# int recv(
#   SOCKET s,
#   char   *buf,
#   int    len,
#   int    flags
# );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
recv = ctypes.windll.ws2_32.recv
recv.argtypes = [
    ctypes.wintypes.HANDLE,
    ctypes.POINTER(ctypes.c_char),
    ctypes.c_int,
    ctypes.c_int,
]
recv.restype = ctypes.c_int

# int closesocket(
#   IN SOCKET s
# );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
closesocket = ctypes.windll.ws2_32.closesocket
closesocket.argtypes = [ctypes.wintypes.HANDLE]
closesocket.restype = ctypes.c_int

# int WSAGetLastError();
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
WSAGetLastError = ctypes.windll.ws2_32.WSAGetLastError
WSAGetLastError.argtypes = []
WSAGetLastError.restype = ctypes.c_int

# setsockopt but "falsely" declared to accept DWORD* as
# its parameter.  It's really char*, but we only use DWORD
# values.
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
WinSetIntSockOpt = ctypes.windll.ws2_32.setsockopt
WinSetIntSockOpt.argtypes = [
    ctypes.wintypes.HANDLE,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.POINTER(ctypes.wintypes.DWORD),
    ctypes.c_int,
]
WinSetIntSockOpt.restype = ctypes.c_int

# int ioctlsocket(
#   [in]      SOCKET s,
#   [in]      long   cmd,
#   [in, out] u_long *argp
# );
# pyre-fixme[5]: Global expression must be annotated.
# pyre-fixme[16]: Module `ctypes` has no attribute `windll`.
ioctlsocket = ctypes.windll.ws2_32.ioctlsocket
ioctlsocket.argtypes = [
    ctypes.wintypes.HANDLE,
    ctypes.c_long,
    ctypes.POINTER(ctypes.wintypes.DWORD),
]
ioctlsocket.restype = ctypes.c_int


class SOCKADDR_UN(ctypes.Structure):
    # pyre-fixme[4]: Attribute must be annotated.
    _fields_ = [("sun_family", ctypes.c_ushort), ("sun_path", ctypes.c_char * 108)]


class WindowsSocketException(Exception):
    def __init__(self, code: int) -> None:
        super(WindowsSocketException, self).__init__(
            "Windows Socket Error: {}".format(code)
        )


class WindowsSocketHandle:
    AF_UNIX = 1
    SOCK_STREAM = 1
    IPPROTO_TCP = 6

    fd: int = -1
    address: str = ""

    @staticmethod
    # pyre-fixme[2]: Parameter must be annotated.
    def _checkReturnCode(retcode) -> None:
        if retcode == -1:
            errcode = WSAGetLastError()
            if errcode == WSAECONNREFUSED:
                raise OSError(
                    errno.ECONNREFUSED,
                    "Windows UDS: Connection refused",
                )
            elif errcode == WSAETIMEDOUT:
                raise TimeoutError(errno.ETIMEDOUT, "Windows UDS: Socket timeout")
            elif errcode == WSAEWOULDBLOCK:
                raise OSError(
                    errno.EWOULDBLOCK,
                    "Windows UDS: Resource temporarily unavailable",
                )
            elif errcode == WSATRY_AGAIN:
                raise OSError(
                    errno.EAGAIN,
                    "Windows UDS: Resource temporarily unavailable",
                )
            else:
                raise WindowsSocketException(errcode)

    def __init__(self) -> None:
        self._io_refs = 0  # stub to make socket.makefile work on this object
        wsa_data = WSAData64()
        # ctypes.c_ushort(514) = MAKE_WORD(2,2) which is for the winsock
        # library version 2.2
        errcode = WSAStartup(ctypes.c_ushort(514), ctypes.pointer(wsa_data))
        if errcode != 0:
            raise WindowsSocketException(errcode)

        fd = socket(self.AF_UNIX, self.SOCK_STREAM, 0)
        self._checkReturnCode(fd)
        self.fd = fd
        # pyre-fixme[4]: Attribute must be annotated.
        self.type = self.SOCK_STREAM
        # pyre-fixme[4]: Attribute must be annotated.
        self.family = self.AF_UNIX
        # pyre-fixme[4]: Attribute must be annotated.
        self.proto = self.IPPROTO_TCP

    def fileno(self) -> int:
        return self.fd

    def settimeout(self, timeout: int) -> None:
        # pyre-fixme[9]: timeout has type `int`; used as `c_ulong`.
        timeout = ctypes.wintypes.DWORD(0 if timeout is None else int(timeout * 1000))
        retcode = WinSetIntSockOpt(
            self.fd,
            SOL_SOCKET,
            SO_RCVTIMEO,
            # pyre-fixme[6]: For 1st argument expected `_CData` but got `int`.
            ctypes.byref(timeout),
            # pyre-fixme[6]: For 1st argument expected `Union[Type[_CData], _CData]`
            #  but got `int`.
            ctypes.sizeof(timeout),
        )
        self._checkReturnCode(retcode)
        retcode = WinSetIntSockOpt(
            self.fd,
            SOL_SOCKET,
            SO_SNDTIMEO,
            # pyre-fixme[6]: For 1st argument expected `_CData` but got `int`.
            ctypes.byref(timeout),
            # pyre-fixme[6]: For 1st argument expected `Union[Type[_CData], _CData]`
            #  but got `int`.
            ctypes.sizeof(timeout),
        )
        self._checkReturnCode(retcode)
        return None

    def connect(self, address: str) -> None:
        addr = SOCKADDR_UN(sun_family=self.AF_UNIX, sun_path=address.encode("utf-8"))
        self._checkReturnCode(
            connect(self.fd, ctypes.pointer(addr), ctypes.sizeof(addr))
        )
        self.address = address

    def bind(self, address: str) -> None:
        addr = SOCKADDR_UN(sun_family=self.AF_UNIX, sun_path=address.encode("utf-8"))
        self._checkReturnCode(bind(self.fd, ctypes.pointer(addr), ctypes.sizeof(addr)))
        self.address = address

    # pyre-fixme[24]: Generic type `memoryview` expects 1 type parameter.
    def send(self, buff: "bytes | memoryview") -> int:
        size = len(buff)

        if isinstance(buff, memoryview):
            # making a copy of buff, because it's not possible to get
            # c_char_p from memoryview (it might not be continuous)
            buff = buff.tobytes()  # making a copy of buff

        retcode = send(self.fd, buff, size, 0)
        self._checkReturnCode(retcode)
        return retcode

    def sendall(self, buff: bytes) -> None:
        while len(buff) > 0:
            x = self.send(buff)
            if x > 0:
                buff = buff[x:]
            else:
                break
        return None

    def recv(self, size: int) -> bytes:
        buff = ctypes.create_string_buffer(size)
        retsize = recv(self.fd, buff, size, 0)
        self._checkReturnCode(retsize)
        return buff.raw[0:retsize]

    # pyre-fixme[3]: Return type must be annotated.
    def recv_into(self, buffer: Buffer, size: int = 0):
        if size == 0:
            # pyre-fixme[6]: For 1st argument expected
            #  `pyre_extensions.PyreReadOnly[Sized]` but got `Buffer`.
            size = len(buffer)

        dest = (ctypes.c_char * size).from_buffer(buffer)
        retsize = recv(self.fd, dest, size, 0)
        self._checkReturnCode(retsize)

        return retsize

    def getpeername(self) -> str:
        return self.address

    def getsockname(self) -> str:
        return self.address

    def close(self) -> int:
        return closesocket(self.fd)

    def setblocking(self, flag: bool) -> int:
        mode = ctypes.wintypes.DWORD(0 if flag else 1)
        retcode = ioctlsocket(self.fd, FIONBIO, ctypes.pointer(mode))
        self._checkReturnCode(retcode)
        return retcode


class WinTSocket(TSocket):
    @property
    def _shouldUseWinsocket(self) -> bool:
        return sys.platform == "win32" and self._unix_socket

    def open(self) -> None:
        # if we are not on Windows or the socktype is not unix socket, return
        # the parent TSocket
        if not self._shouldUseWinsocket:
            return super(WinTSocket, self).open()

        handle = WindowsSocketHandle()
        self.setHandle(handle)
        handle.settimeout(self._timeout)
        try:
            handle.connect(self._unix_socket)
        except OSError as e:
            self.close()
            if e.errno == errno.ECONNREFUSED:
                # This error will be returned when Edenfs is not running
                raise TTransportException(
                    type=TTransportException.NOT_OPEN, message="eden not running"
                )
            raise e
        except Exception:
            self.close()
            raise

import time

import pyotp


class Authenticator:
    def __init__(self, totp_uri: str) -> None:
        """
        Initialize an Authenticator.

        :param totp_uri: TOTP URI
        :type totp_uri: str
        :raises ValueError: if the URI is not valid
        """
        otp = pyotp.parse_uri(totp_uri)
        if not isinstance(otp, pyotp.TOTP):
            raise ValueError("URI must be for TOTP")
        self._totp = otp

        self._used_otps: dict[str, float] = {}

    def verify(self, otp: str) -> bool:
        """
        Verify if the sender of the OTP is authorized

        :param otp: the OTP received
        :type otp: str
        :return: True if the sender is authorized, False otherwise
        :rtype: bool
        """
        if self._is_used(otp):
            return False

        if self._totp.verify(otp):
            # Use OTP if it is valid
            self._use_otp(otp)
            return True

        return False

    def _use_otp(self, otp: str) -> None:
        # Expired until the OTP is no longer valid
        self._used_otps[otp] = time.monotonic() + self._totp.interval

    def _is_used(self, otp: str) -> bool:
        expired_until = self._used_otps.get(otp)
        if expired_until is None:
            return False

        return time.monotonic() > expired_until

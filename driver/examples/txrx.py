import argparse
import cantact
from typing import List, Dict, Tuple, Optional, Union, NamedTuple


class CANInterfaceSingleton:
    _instance = None

    def __new__(cls, *args, **kwargs):
        if not cls._instance:
            cls._instance = super().__new__(cls, *args, **kwargs)
            cls._instance = cantact.Interface()
        return cls._instance


class CANTransmitter:

    def __init__(self, device_id: int):
        self.cantact_interface = CANInterfaceSingleton()
        self.device_id = device_id
        self.tx_count = 0

    def start(self, arbitration_speed: int, data_speed: Optional[int] = None):
        self.cantact_interface.stop()
        self.cantact_interface.set_bitrate(self.device_id, arbitration_speed)
        if data_speed is not None:
            self.cantact_interface.set_data_bitrate(self.device_id, data_speed)
        self.cantact_interface.set_enabled(self.device_id, True)
        self.cantact_interface.start()

    def stop(self):
        self.cantact_interface.stop()
        self.cantact_interface.set_enabled(self.device_id, False)
        self.cantact_interface.start()

    def send(self, tx_data: List[int], arbitration: int = 0x01B, fd: bool = False, brs: bool = False):
        self.tx_count += 1
        self.cantact_interface.send_fd(self.device_id, arbitration, False, False, fd, brs, len(tx_data), tx_data)
        print(self.tx_count)

    def recv(self):
        self.cantact_interface.recv(self.device_id)


if "__main__" == __name__:
    arg_parser = argparse.ArgumentParser(prog="CAN Tx Rx", description="Transmit CAN message using cantact")
    arg_parser.add_argument("data", nargs="+", type=int, help="Data to be transmitted in CAN")
    arg_parser.add_argument("-t", "--tx-channel", nargs="?", type=int, default=0)
    arg_parser.add_argument("-r", "--rx-channel", nargs="?", type=int, default=1)
    arg_parser.add_argument("-s", "--speed-for-arbitration", nargs="?", type=int, default=500_000)
    arg_parser.add_argument("-d", "--speed-for-data", nargs="?", type=int, default=500_000)
    args = arg_parser.parse_args()
    can_if = CANInterfaceSingleton()
    can_tx = CANTransmitter(args.tx_channel)
    fd = (args.speed_for_data > args.speed_for_arbitration) and (args.speed_for_data > 500_000)
    can_tx.start(args.speed_for_arbitration, args.speed_for_data if fd else None)
    can_tx.send(args.data, fd=fd, brs=fd)
    recv = can_if.recv(args.rx_channel)
    if recv:
        print(recv)

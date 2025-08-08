#ifndef CANTACT_H_
#define CANTACT_H_

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus
	#include <stdint.h>

	typedef void* cantacthnd;

	struct CantactFrame {
		uint8_t channel;
		uint32_t id;
		uint8_t dlc;
		uint8_t data[64];
		uint8_t ext;
		uint8_t fd;
		uint8_t brs;
		uint8_t esi;
		uint8_t loopback;
		uint8_t rtr;
		uint8_t err;
	};

	typedef void(__cdecl* CantactRxCallback)(const struct CantactFrame* f);

	__declspec(dllimport) cantacthnd cantact_init();
	__declspec(dllimport) int32_t cantact_deinit(cantacthnd hnd);

	__declspec(dllimport) int32_t cantact_open(cantacthnd hnd);
	__declspec(dllimport) int32_t cantact_close(cantacthnd hnd);

	__declspec(dllimport) int32_t cantact_set_rx_callback(cantacthnd hnd, CantactRxCallback callback);

	__declspec(dllimport) int32_t cantact_start(cantacthnd hnd);
	__declspec(dllimport) int32_t cantact_stop(cantacthnd hnd);

	__declspec(dllimport) int32_t cantact_transmit(cantacthnd hnd, const struct CantactFrame f);

	__declspec(dllimport) int32_t cantact_set_bitrate(cantacthnd hnd, uint8_t channel, uint32_t bitrate);
	__declspec(dllimport) int32_t cantact_set_data_bitrate(cantacthnd hnd, uint8_t channel, uint32_t bitrate);
	__declspec(dllimport) int32_t cantact_set_enabled(cantacthnd hnd, uint8_t channel, uint8_t enabled);
	__declspec(dllimport) int32_t cantact_set_monitor(cantacthnd hnd, uint8_t channel, uint8_t enabled);
	__declspec(dllimport) int32_t cantact_set_hw_loopback(cantacthnd hnd, uint8_t channel, uint8_t enabled);

	__declspec(dllimport) int32_t cantact_get_channel_count(cantacthnd hnd);
#ifdef __cplusplus
}
#endif  // __cplusplus

#endif

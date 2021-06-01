#define MS_CLASS "DepLibUV"
// #define MS_LOG_DEV_LEVEL 3

#include "DepLibUV.hpp"
#include "Logger.hpp"
#include <cstdlib> // std::abort()

#include <iostream>
#include <fstream>

using std::cout; using std::ofstream;
using std::endl; using std::string;
using std::cerr;
using std::fstream;

/* Static variables. */

thread_local uv_loop_t* DepLibUV::loop{ nullptr };

/* Static methods for UV callbacks. */

inline static void onClose(uv_handle_t* handle)
{
	delete handle;
}

inline static void onWalk(uv_handle_t* handle, void* arg)
{
	MS_DUMP(
		"---- handle [type:%d, active:%d, closing:%d, has_ref:%d]",
		handle->type,
		uv_is_active(handle),
		uv_is_closing(handle),
		uv_has_ref(handle)
	);

	string filename("/tmp/kk.txt");
	fstream output_fstream;

	output_fstream.open(filename, std::ios_base::app);
	output_fstream << "handle type:" << handle->type << endl;

	if (!uv_is_closing(handle))
		uv_close(handle, onClose);
}

/* Static methods. */

void DepLibUV::ClassInit()
{
	// NOTE: Logger depends on this so we cannot log anything here.

	DepLibUV::loop = new uv_loop_t;

	int err = uv_loop_init(DepLibUV::loop);

	if (err != 0)
		MS_ABORT("libuv initialization failed");
}

void DepLibUV::ClassDestroy()
{
	MS_TRACE();

	// This should never happen.
	if (DepLibUV::loop != nullptr)
	{
		// uv_loop_close(DepLibUV::loop);
		// delete DepLibUV::loop;


		int err;

		uv_stop(DepLibUV::loop);
		uv_walk(DepLibUV::loop, onWalk, nullptr);

		while (true)
		{
			err = uv_loop_close(DepLibUV::loop);

			if (err != UV_EBUSY)
				break;

			uv_run(DepLibUV::loop, UV_RUN_NOWAIT);
		}

		if (err != 0)
			MS_ABORT("failed to close libuv loop: %s", uv_err_name(err));

		delete DepLibUV::loop;
	}
}

void DepLibUV::PrintVersion()
{
	MS_TRACE();

	MS_DEBUG_TAG(info, "libuv version: \"%s\"", uv_version_string());
}

void DepLibUV::RunLoop()
{
	MS_TRACE();

	// This should never happen.
	MS_ASSERT(DepLibUV::loop != nullptr, "loop unset");

	int ret = uv_run(DepLibUV::loop, UV_RUN_DEFAULT);

	MS_ASSERT(ret == 0, "uv_run() returned %s", uv_err_name(ret));
}

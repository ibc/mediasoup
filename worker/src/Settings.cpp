#define MS_CLASS "Settings"
// #define MS_LOG_DEV_LEVEL 3

#include "Settings.hpp"
#include "Logger.hpp"
#include "MediaSoupErrors.hpp"
#include "Utils.hpp"
#include <cctype> // isprint()
#include <cerrno>
#include <iterator> // std::ostream_iterator
#include <mutex>
#include <nlohmann/json.hpp>
#include <sstream> // std::ostringstream
extern "C"
{
#include <getopt.h>
}

/* Static. */

static std::mutex globalSyncMutex;

/* Class variables. */

thread_local struct Settings::Configuration Settings::configuration;
// clang-format off
std::map<std::string, LogLevel> Settings::string2LogLevel =
{
	{ "debug", LogLevel::LOG_DEBUG },
	{ "warn",  LogLevel::LOG_WARN  },
	{ "error", LogLevel::LOG_ERROR },
	{ "none",  LogLevel::LOG_NONE  }
};
std::map<LogLevel, std::string> Settings::logLevel2String =
{
	{ LogLevel::LOG_DEBUG, "debug" },
	{ LogLevel::LOG_WARN,  "warn"  },
	{ LogLevel::LOG_ERROR, "error" },
	{ LogLevel::LOG_NONE,  "none"  }
};
// clang-format on

/* Class methods. */

void Settings::SetConfiguration(int argc, char* argv[])
{
	MS_TRACE();

	/* Variables for getopt. */

	int c;
	int optionIdx{ 0 };
	// clang-format off
	struct option options[] =
	{
		{ "logLevel",            optional_argument, nullptr, 'l' },
		{ "logTags",             optional_argument, nullptr, 't' },
		{ "rtcMinPort",          optional_argument, nullptr, 'm' },
		{ "rtcMaxPort",          optional_argument, nullptr, 'M' },
		{ "dtlsCertificateFile", optional_argument, nullptr, 'c' },
		{ "dtlsPrivateKeyFile",  optional_argument, nullptr, 'p' },
		{ "sendNackDelayMs",     optional_argument, nullptr, 'n' },
		{ nullptr, 0, nullptr, 0 }
	};
	// clang-format on
	std::string stringValue;
	std::vector<std::string> logTags;

	/* Parse command line options. */

	// getopt_long_only() is not thread-safe
	std::lock_guard<std::mutex> lock(globalSyncMutex);

	optind = 1; // Set explicitly, otherwise subsequent runs will fail.
	opterr = 0; // Don't allow getopt to print error messages.
	while ((c = getopt_long_only(argc, argv, "", options, &optionIdx)) != -1)
	{
		if (!optarg)
			MS_THROW_TYPE_ERROR("unknown configuration parameter: %s", optarg);

		switch (c)
		{
			case 'l':
			{
				stringValue = std::string(optarg);
				SetLogLevel(stringValue);

				break;
			}

			case 't':
			{
				stringValue = std::string(optarg);
				logTags.push_back(stringValue);

				break;
			}

			case 'm':
			{
				try
				{
					Settings::configuration.rtcMinPort = static_cast<uint16_t>(std::stoi(optarg));
				}
				catch (const std::exception& error)
				{
					MS_THROW_TYPE_ERROR("%s", error.what());
				}

				break;
			}

			case 'M':
			{
				try
				{
					Settings::configuration.rtcMaxPort = static_cast<uint16_t>(std::stoi(optarg));
				}
				catch (const std::exception& error)
				{
					MS_THROW_TYPE_ERROR("%s", error.what());
				}

				break;
			}

			case 'c':
			{
				stringValue                                 = std::string(optarg);
				Settings::configuration.dtlsCertificateFile = stringValue;

				break;
			}

			case 'p':
			{
				stringValue                                = std::string(optarg);
				Settings::configuration.dtlsPrivateKeyFile = stringValue;
				break;
			}

			case 'n':
			{
				try
				{
					Settings::configuration.sendNackDelayMs = std::stoull(optarg);
				}
				catch (const std::exception& error)
				{
					MS_THROW_TYPE_ERROR("%s", error.what());
				}
				break;
			}

			// Invalid option.
			case '?':
			{
				if (isprint(optopt) != 0)
					MS_THROW_TYPE_ERROR("invalid option '-%c'", (char)optopt);
				else
					MS_THROW_TYPE_ERROR("unknown long option given as argument");
			}

			// Valid option, but it requires and argument that is not given.
			case ':':
			{
				MS_THROW_TYPE_ERROR("option '%c' requires an argument", (char)optopt);
			}

			// This should never happen.
			default:
			{
				MS_THROW_TYPE_ERROR("'default' should never happen");
			}
		}
	}

	/* Post configuration. */

	// Set logTags.
	if (!logTags.empty())
		Settings::SetLogTags(logTags);

	// Validate RTC ports.
	if (Settings::configuration.rtcMaxPort < Settings::configuration.rtcMinPort)
		MS_THROW_TYPE_ERROR("rtcMaxPort cannot be less than rtcMinPort");

	// Set DTLS certificate files (if provided),
	Settings::SetDtlsCertificateAndPrivateKeyFiles();
}

void Settings::PrintConfiguration()
{
	MS_TRACE();

	std::vector<std::string> logTags;
	std::ostringstream logTagsStream;

	if (Settings::configuration.logTags.info)
		logTags.emplace_back("info");
	if (Settings::configuration.logTags.ice)
		logTags.emplace_back("ice");
	if (Settings::configuration.logTags.dtls)
		logTags.emplace_back("dtls");
	if (Settings::configuration.logTags.rtp)
		logTags.emplace_back("rtp");
	if (Settings::configuration.logTags.srtp)
		logTags.emplace_back("srtp");
	if (Settings::configuration.logTags.rtcp)
		logTags.emplace_back("rtcp");
	if (Settings::configuration.logTags.rtx)
		logTags.emplace_back("rtx");
	if (Settings::configuration.logTags.bwe)
		logTags.emplace_back("bwe");
	if (Settings::configuration.logTags.score)
		logTags.emplace_back("score");
	if (Settings::configuration.logTags.simulcast)
		logTags.emplace_back("simulcast");
	if (Settings::configuration.logTags.svc)
		logTags.emplace_back("svc");
	if (Settings::configuration.logTags.sctp)
		logTags.emplace_back("sctp");
	if (Settings::configuration.logTags.message)
		logTags.emplace_back("message");

	if (!logTags.empty())
	{
		std::copy(
		  logTags.begin(), logTags.end() - 1, std::ostream_iterator<std::string>(logTagsStream, ","));
		logTagsStream << logTags.back();
	}

	MS_DEBUG_TAG(info, "<configuration>");

	MS_DEBUG_TAG(
	  info,
	  "  logLevel            : %s",
	  Settings::logLevel2String[Settings::configuration.logLevel].c_str());
	MS_DEBUG_TAG(info, "  logTags             : %s", logTagsStream.str().c_str());
	MS_DEBUG_TAG(info, "  rtcMinPort          : %" PRIu16, Settings::configuration.rtcMinPort);
	MS_DEBUG_TAG(info, "  rtcMaxPort          : %" PRIu16, Settings::configuration.rtcMaxPort);
	if (!Settings::configuration.dtlsCertificateFile.empty())
	{
		MS_DEBUG_TAG(
		  info, "  dtlsCertificateFile : %s", Settings::configuration.dtlsCertificateFile.c_str());
		MS_DEBUG_TAG(
		  info, "  dtlsPrivateKeyFile  : %s", Settings::configuration.dtlsPrivateKeyFile.c_str());
	}

	MS_DEBUG_TAG(info, "</configuration>");
}

void Settings::HandleRequest(Channel::ChannelRequest* request)
{
	MS_TRACE();

	switch (request->methodId)
	{
		case Channel::ChannelRequest::MethodId::WORKER_UPDATE_SETTINGS:
		{
			auto jsonLogLevelIt = request->data.find("logLevel");
			auto jsonLogTagsIt  = request->data.find("logTags");

			// Update logLevel if requested.
			if (jsonLogLevelIt != request->data.end() && jsonLogLevelIt->is_string())
			{
				std::string logLevel = *jsonLogLevelIt;

				// This may throw.
				Settings::SetLogLevel(logLevel);
			}

			// Update logTags if requested.
			if (jsonLogTagsIt != request->data.end() && jsonLogTagsIt->is_array())
			{
				std::vector<std::string> logTags;

				for (const auto& logTag : *jsonLogTagsIt)
				{
					if (logTag.is_string())
						logTags.push_back(logTag);
				}

				Settings::SetLogTags(logTags);
			}

			// Print the new effective configuration.
			Settings::PrintConfiguration();

			request->Accept();

			break;
		}

		default:
		{
			MS_THROW_ERROR("unknown method '%s'", request->method.c_str());
		}
	}
}

void Settings::SetLogLevel(std::string& level)
{
	MS_TRACE();

	// Lowcase given level.
	Utils::String::ToLowerCase(level);

	if (Settings::string2LogLevel.find(level) == Settings::string2LogLevel.end())
		MS_THROW_TYPE_ERROR("invalid value '%s' for logLevel", level.c_str());

	Settings::configuration.logLevel = Settings::string2LogLevel[level];
}

void Settings::SetLogTags(const std::vector<std::string>& tags)
{
	MS_TRACE();

	// Reset logTags.
	struct LogTags newLogTags;

	for (auto& tag : tags)
	{
		if (tag == "info")
			newLogTags.info = true;
		else if (tag == "ice")
			newLogTags.ice = true;
		else if (tag == "dtls")
			newLogTags.dtls = true;
		else if (tag == "rtp")
			newLogTags.rtp = true;
		else if (tag == "srtp")
			newLogTags.srtp = true;
		else if (tag == "rtcp")
			newLogTags.rtcp = true;
		else if (tag == "rtx")
			newLogTags.rtx = true;
		else if (tag == "bwe")
			newLogTags.bwe = true;
		else if (tag == "score")
			newLogTags.score = true;
		else if (tag == "simulcast")
			newLogTags.simulcast = true;
		else if (tag == "svc")
			newLogTags.svc = true;
		else if (tag == "sctp")
			newLogTags.sctp = true;
		else if (tag == "message")
			newLogTags.message = true;
	}

	Settings::configuration.logTags = newLogTags;
}

void Settings::SetDtlsCertificateAndPrivateKeyFiles()
{
	MS_TRACE();

	if (
	  !Settings::configuration.dtlsCertificateFile.empty() &&
	  Settings::configuration.dtlsPrivateKeyFile.empty())
	{
		MS_THROW_TYPE_ERROR("missing dtlsPrivateKeyFile");
	}
	else if (
	  Settings::configuration.dtlsCertificateFile.empty() &&
	  !Settings::configuration.dtlsPrivateKeyFile.empty())
	{
		MS_THROW_TYPE_ERROR("missing dtlsCertificateFile");
	}
	else if (
	  Settings::configuration.dtlsCertificateFile.empty() &&
	  Settings::configuration.dtlsPrivateKeyFile.empty())
	{
		return;
	}

	const std::string& dtlsCertificateFile = Settings::configuration.dtlsCertificateFile;
	const std::string& dtlsPrivateKeyFile  = Settings::configuration.dtlsPrivateKeyFile;

	try
	{
		Utils::File::CheckFile(dtlsCertificateFile.c_str());
	}
	catch (const MediaSoupError& error)
	{
		MS_THROW_TYPE_ERROR("dtlsCertificateFile: %s", error.what());
	}

	try
	{
		Utils::File::CheckFile(dtlsPrivateKeyFile.c_str());
	}
	catch (const MediaSoupError& error)
	{
		MS_THROW_TYPE_ERROR("dtlsPrivateKeyFile: %s", error.what());
	}
}

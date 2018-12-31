#define MS_CLASS "Settings"
// #define MS_LOG_DEV

#include "Settings.hpp"
#include "Logger.hpp"
#include "MediaSoupError.hpp"
#include "Utils.hpp"
#include "json.hpp"
#include <cctype> // isprint()
#include <cerrno>
#include <iterator> // std::ostream_iterator
#include <sstream>  // std::ostringstream
extern "C" {
#include <getopt.h>
}

/* Class variables. */

struct Settings::Configuration Settings::configuration;
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
		{ nullptr, 0, nullptr, 0 }
	};
	// clang-format on
	std::string stringValue;
	std::vector<std::string> logTags;

	/* Parse command line options. */

	opterr = 0; // Don't allow getopt to print error messages.
	while ((c = getopt_long_only(argc, argv, "", options, &optionIdx)) != -1)
	{
		if (optarg == nullptr)
			MS_THROW_ERROR("unknown configuration parameter: %s", optarg);

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
					MS_THROW_ERROR("%s", error.what());
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
					MS_THROW_ERROR("%s", error.what());
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

			// Invalid option.
			case '?':
			{
				if (isprint(optopt) != 0)
					MS_THROW_ERROR("invalid option '-%c'", (char)optopt);
				else
					MS_THROW_ERROR("unknown long option given as argument");
			}

			// Valid option, but it requires and argument that is not given.
			case ':':
			{
				MS_THROW_ERROR("option '%c' requires an argument", (char)optopt);
			}

			// This should never happen.
			default:
			{
				MS_THROW_ERROR("'default' should never happen");
			}
		}
	}

	/* Post configuration. */

	// Set logTags.
	if (!logTags.empty())
		Settings::SetLogTags(logTags);

	// Validate RTC ports.
	if (Settings::configuration.rtcMaxPort <= Settings::configuration.rtcMinPort)
		MS_THROW_ERROR("rtcMaxPort must be higher than rtcMinPort");

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
	if (Settings::configuration.logTags.rbe)
		logTags.emplace_back("rbe");
	if (Settings::configuration.logTags.tmp)
		logTags.emplace_back("tmp");

	if (!logTags.empty())
	{
		std::copy(
		  logTags.begin(), logTags.end() - 1, std::ostream_iterator<std::string>(logTagsStream, ","));
		logTagsStream << logTags.back();
	}

	MS_DEBUG_TAG(info, "<configuration>");

	MS_DEBUG_TAG(
	  info,
	  "  logLevel            : \"%s\"",
	  Settings::logLevel2String[Settings::configuration.logLevel].c_str());
	MS_DEBUG_TAG(info, "  logTags             : \"%s\"", logTagsStream.str().c_str());
	MS_DEBUG_TAG(info, "  rtcMinPort          : %" PRIu16, Settings::configuration.rtcMinPort);
	MS_DEBUG_TAG(info, "  rtcMaxPort          : %" PRIu16, Settings::configuration.rtcMaxPort);
	if (!Settings::configuration.dtlsCertificateFile.empty())
	{
		MS_DEBUG_TAG(
		  info, "  dtlsCertificateFile : \"%s\"", Settings::configuration.dtlsCertificateFile.c_str());
		MS_DEBUG_TAG(
		  info, "  dtlsPrivateKeyFile  : \"%s\"", Settings::configuration.dtlsPrivateKeyFile.c_str());
	}

	MS_DEBUG_TAG(info, "</configuration>");
}

void Settings::HandleRequest(Channel::Request* request)
{
	MS_TRACE();

	switch (request->methodId)
	{
		case Channel::Request::MethodId::WORKER_UPDATE_SETTINGS:
		{
			auto jsonLogLevelIt = request->data.find("logLevel");
			auto jsonLogTagsIt  = request->data.find("logTags");

			try
			{
				// Update logLevel if requested.
				if (jsonLogLevelIt != request->data.end() && jsonLogLevelIt->is_string())
				{
					std::string logLevel = *jsonLogLevelIt;

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
			}
			catch (const MediaSoupError& error)
			{
				request->Reject(error.what());

				return;
			}

			// Print the new effective configuration.
			Settings::PrintConfiguration();

			request->Accept();

			break;
		}

		default:
		{
			MS_ERROR("unknown method");

			request->Reject("unknown method");
		}
	}
}

void Settings::SetLogLevel(std::string& level)
{
	MS_TRACE();

	// Lowcase given level.
	Utils::String::ToLowerCase(level);

	if (Settings::string2LogLevel.find(level) == Settings::string2LogLevel.end())
		MS_THROW_ERROR("invalid value '%s' for logLevel", level.c_str());

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
		else if (tag == "rbe")
			newLogTags.rbe = true;
		else if (tag == "tmp")
			newLogTags.tmp = true;
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
		MS_THROW_ERROR("missing dtlsPrivateKeyFile");
	}
	else if (
	  Settings::configuration.dtlsCertificateFile.empty() &&
	  !Settings::configuration.dtlsPrivateKeyFile.empty())
	{
		MS_THROW_ERROR("missing dtlsCertificateFile");
	}
	else if (
	  Settings::configuration.dtlsCertificateFile.empty() &&
	  Settings::configuration.dtlsPrivateKeyFile.empty())
	{
		return;
	}

	std::string& dtlsCertificateFile = Settings::configuration.dtlsCertificateFile;
	std::string& dtlsPrivateKeyFile  = Settings::configuration.dtlsPrivateKeyFile;

	try
	{
		Utils::File::CheckFile(dtlsCertificateFile.c_str());
	}
	catch (const MediaSoupError& error)
	{
		MS_THROW_ERROR("dtlsCertificateFile: %s", error.what());
	}

	try
	{
		Utils::File::CheckFile(dtlsPrivateKeyFile.c_str());
	}
	catch (const MediaSoupError& error)
	{
		MS_THROW_ERROR("dtlsPrivateKeyFile: %s", error.what());
	}

	Settings::configuration.dtlsCertificateFile = dtlsCertificateFile;
	Settings::configuration.dtlsPrivateKeyFile  = dtlsPrivateKeyFile;
}

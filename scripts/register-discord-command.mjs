#!/usr/bin/env node

const apiBase = process.env.DISCORD_API_BASE ?? "https://discord.com/api/v10";
const appId = process.env.DISCORD_APP_ID;
const botToken = process.env.DISCORD_BOT_TOKEN;
const guildId = process.env.DISCORD_GUILD_ID;

const args = new Set(process.argv.slice(2));
const dryRun = args.has("--dry-run");
const deleteCommand = args.has("--delete");
const commandName = [...args].find((arg) => !arg.startsWith("--")) ?? "jbotci";

function stringOption(name, description, required = false) {
  return { name, description, type: 3, required };
}

function integerOption(name, description, required = false) {
  return { name, description, type: 4, required, min_value: 1 };
}

function boolOption(name, description, required = false) {
  return { name, description, type: 5, required };
}

function choiceOption(name, description, choices, required = false) {
  return {
    name,
    description,
    type: 3,
    required,
    choices: choices.map((choice) => ({ name: choice, value: choice })),
  };
}

const payload = {
  name: commandName,
  type: 1,
  description: "Lojban tools",
  options: [
    {
      name: "gentufa",
      description: "Parse Lojban syntax",
      type: 1,
      options: [
        stringOption("text", "Lojban text to parse", true),
        choiceOption("format", "Output format", ["tree", "brackets", "json", "svg", "png"]),
        stringOption("dialect", "Dialect formula"),
        boolOption("show-elided", "Show elided terminators"),
      ],
    },
    {
      name: "vlasei",
      description: "Segment Lojban morphology",
      type: 1,
      options: [
        stringOption("text", "Lojban text to segment", true),
        choiceOption("format", "Output format", ["tree", "brackets", "ipa", "json"]),
      ],
    },
    {
      name: "vlacku",
      description: "Search dictionary cards",
      type: 1,
      options: [
        stringOption("query", "Search query", true),
        choiceOption("mode", "Search mode", [
          "word",
          "rafsi",
          "lujvo",
          "sound",
          "meaning",
          "regex",
          "rafsi-regex",
          "glob",
          "rafsi-glob",
        ]),
        integerOption("count", "Maximum result count"),
        stringOption("word-type", "Word type filter"),
      ],
    },
    {
      name: "cukta",
      description: "Read or search the CLL",
      type: 1,
      options: [
        choiceOption("mode", "Read/search mode", [
          "default",
          "toc",
          "section",
          "example",
          "word",
          "meaning",
        ]),
        stringOption("query", "Reference or search query"),
        integerOption("count", "Maximum result count"),
        choiceOption("format", "Output format", ["markdown", "html", "raw"]),
      ],
    },
    {
      name: "jvozba",
      description: "Build a lujvo or cmevla",
      type: 1,
      options: [
        stringOption("parts", "Space-separated source words", true),
        stringOption("rafsi", "Space- or comma-separated fixed rafsi appended after words"),
        choiceOption("mode", "Output word kind", ["lujvo", "cmevla"]),
      ],
    },
    {
      name: "gimfi'i",
      description: "Compose candidate gismu",
      type: 1,
      options: [
        stringOption("sources", "Comma-separated LANG[:WEIGHT]:WORD source specs"),
        stringOption("preset", "Preset source set"),
        integerOption("count", "Maximum candidate count"),
        choiceOption("format", "Output format", ["table", "json"]),
      ],
    },
  ],
};

if (!appId) {
  throw new Error("DISCORD_APP_ID is required");
}
if (!dryRun && !botToken) {
  throw new Error("DISCORD_BOT_TOKEN is required unless --dry-run is used");
}

const endpoint = guildId
  ? `${apiBase}/applications/${appId}/guilds/${guildId}/commands`
  : `${apiBase}/applications/${appId}/commands`;

async function discordRequest(method, url, body) {
  if (dryRun) {
    console.log(JSON.stringify({ method, url, body }, null, 2));
    return undefined;
  }
  const response = await fetch(url, {
    method,
    headers: {
      Authorization: `Bot ${botToken}`,
      "Content-Type": "application/json",
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`${method} ${url} failed: ${response.status} ${text}`);
  }
  return text ? JSON.parse(text) : undefined;
}

if (deleteCommand) {
  const commands = dryRun ? [] : await discordRequest("GET", endpoint);
  const existing = commands.find((command) => command.name === commandName);
  if (!existing) {
    if (dryRun) {
      console.log(JSON.stringify({ method: "DELETE", url: `${endpoint}/<${commandName}-id>` }, null, 2));
    } else {
      console.log(`No existing /${commandName} command found.`);
    }
  } else {
    await discordRequest("DELETE", `${endpoint}/${existing.id}`);
    console.log(`Deleted /${commandName}.`);
  }
} else {
  await discordRequest("POST", endpoint, payload);
  if (dryRun) {
    console.log(`Dry run: would register /${commandName}${guildId ? ` in guild ${guildId}` : " globally"}.`);
  } else {
    console.log(`Registered /${commandName}${guildId ? ` in guild ${guildId}` : " globally"}.`);
  }
}

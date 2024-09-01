const Fs = require('fs-extra')
const Path = require('path')

const CONFIG_FILE_DIR = Path.resolve(__dirname, 'config')
const CONFIG_FILE = Path.resolve(CONFIG_FILE_DIR, 'protocol.json')

const writeConfig = (config = { registered: false, port: 0 }) => {
  Fs.ensureDirSync(CONFIG_FILE_DIR)
  Fs.writeJsonSync(CONFIG_FILE, config)
  return config
}

const readConfig = () => Fs.readJsonSync(CONFIG_FILE)

const getConfig = () => {
  try {
    return readConfig()
  } catch (error) {
    return writeConfig()
  }
}

const data = getConfig()

module.exports = {
  data,
  modify: obj => writeConfig(Object.assign(data, obj)),
}

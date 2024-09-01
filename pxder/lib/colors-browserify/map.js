'use strict'
// use a style sheet for this read out in the browser
exports.colors = {
  black: 'black',
  red: 'red',
  green: 'green',
  yellow: 'yellow',
  blue: 'blue',
  magenta: 'magenta',
  cyan: 'cyan',
  white: 'white',
  gray: 'gray',
  grey: 'grey'
}

exports.backgrounds = {
  bgBlack: 'black',
  bgRed: 'red',
  bgGreen: 'green',
  bgYellow: 'yellow',
  bgBlue: 'blue',
  bgMagenta: 'magenta',
  bgCyan: 'cyan',
  bgWhite: 'white'
}

exports.styles = {
  reset: '',
  bold: { 'font-weight': 'bold' },
  dim: { opacity: 0.5 },
  italic: { 'font-style': 'italic' },
  underline: { 'text-decoration': 'underline' },
  inverse: { background: '#333' },
  hidden: { 'visibility': 'hidden' },
  strikethrough: { 'text-decoration': 'line-through' }
}

exports.extras = {
  rainbow: {
    //only webkit for now
    'background-image': `
      -webkit-gradient(
        linear,
        left top,
        right top,
        color-stop(0, #f22),
        color-stop(0.15, #f2f),
        color-stop(0.3, #22f),
        color-stop(0.45, #2ff),
        color-stop(0.6, #2f2),
        color-stop(0.75, #2f2),
        color-stop(0.9, #ff2),
        color-stop(1, #f22)
      )
    `,
    color: 'transparent',
    '-webkit-background-clip': 'text',
    'background-clip': 'text'
  },
  zebra: { 'font-style': 'italic' },
  america: { 'font-style': 'italic' },
  trap: { 'font-style': 'italic' },
  random: { 'font-style': 'italic' }
}

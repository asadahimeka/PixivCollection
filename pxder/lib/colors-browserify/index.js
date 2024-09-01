'use strict'
// pretty dirty but makes the colors module compatible with the browser
var map = exports.map = require('./map')
var stringProto = String.prototype

for (let key in map.colors) {
  define(key, 'color:' + map.colors[key] + ';')
}

for (let key in map.backgrounds) {
  define(key, 'background-color:' + map.backgrounds[key] + ';')
}

for (let key in map.styles) {
  define(key, convertToCss(map.styles[key]))
}

for (let key in map.extras) {
  define(key, convertToCss(map.extras[key]))
}

// util functions
function define(key, css) {
  Object.defineProperty(stringProto, key, {
    get() {
      if (/\n/.test(this)) {
        return this.split(/\n/).map(e => '$${%c<' + css + '>%' + e + '}$$__EOS__').join('\n');
      }
      return '$${%c<' + css + '>%' + this + '}$$__EOS__';
    },
    configurable: true
  })
}

function convertToCss(obj) {
  let cssMap = obj
  let css = ''
  for (let style in cssMap) {
    css += style + ':' + cssMap[style] + ';'
  }
  return css
}

'use strict';

// Cloud Function entry point — re-exports handler from functions/
const { handler } = require('./functions');

module.exports = { handler };

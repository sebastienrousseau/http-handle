searchState.loadedDescShard("http_handle", 0, "The <code>error</code> module defines various errors that can occur …\nThe <code>request</code> module is responsible for parsing and …\nThe <code>response</code> module provides tools and utilities for …\nThe <code>server</code> module contains the core <code>Server</code> struct and …\nA custom error type for unexpected scenarios.\nAccess to the requested resource is forbidden.\nThe request received by the server was invalid or …\nAn I/O error occurred.\nThe requested file was not found on the server.\nRepresents the different types of errors that can occur in …\nCreates a new <code>Forbidden</code> error with the given message.\nConverts a string slice into a <code>ServerError::Custom</code> variant.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCreates a new <code>InvalidRequest</code> error with the given message.\nCreates a new <code>NotFound</code> error with the given path.\nRepresents an HTTP request, containing the HTTP method, …\nReturns the argument unchanged.\nAttempts to create a <code>Request</code> from the provided TCP stream …\nCalls <code>U::from(self)</code>.\nReturns the HTTP method of the request.\nHTTP method of the request.\nReturns the requested path of the request.\nRequested path.\nReturns the HTTP version of the request.\nHTTP version of the request.\nRepresents an HTTP response, including the status code, …\nAdds a header to the response.\nThe body of the response, represented as a vector of bytes.\nReturns the argument unchanged.\nA list of headers in the response, each represented as a …\nCalls <code>U::from(self)</code>.\nCreates a new <code>Response</code> with the given status code, status …\nSends the response over the provided <code>Write</code> stream.\nThe HTTP status code (e.g., 200 for OK, 404 for Not Found).\nThe HTTP status text associated with the status code …\nRepresents the Http Handle and its configuration.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCreates a new <code>Server</code> instance.\nStarts the server and begins listening for incoming …")
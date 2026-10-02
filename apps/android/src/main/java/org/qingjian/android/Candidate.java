package org.qingjian.android;

/** JNI 返回的单个中文候选及其英语释义。 */
final class Candidate {
  final String text;
  volatile String gloss;
  final int consumed;

  Candidate(String text, String gloss, int consumed) {
    this.text = text;
    this.gloss = gloss;
    this.consumed = consumed;
  }
}

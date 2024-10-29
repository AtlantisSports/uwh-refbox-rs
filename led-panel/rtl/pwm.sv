`timescale 1ns / 1ps

module pwm #(parameter CTR_LEN = 8, parameter NUM_OUTPUTS = 1) (
    input clk,
    input rst,
    input [CTR_LEN - 1 : 0] compare [NUM_OUTPUTS : 0],
    output [NUM_OUTPUTS - 1 : 0] pwm
  );
 
  reg [NUM_OUTPUTS - 1 : 0] pwm_d, pwm_q;
  reg [CTR_LEN - 1: 0] ctr_d, ctr_q;
 
  assign pwm = pwm_q;
 
  always_comb begin
    ctr_d = ctr_q + 1'b1;
 
    for (int i = 0; i < NUM_OUTPUTS; i = i + 1) begin
        if (ctr_d >= compare[i] && ctr_d < compare[i+1])
            pwm_d[i] = 1'b1;
        else
            pwm_d[i] = 1'b0;
    end
  end
 
  always @(posedge clk) begin
    if (rst) begin
      ctr_q <= 1'b0;
      for (int i = 0; i < NUM_OUTPUTS; i = i + 1) begin
          pwm_q[i] <= 1'b0;
      end
    end else begin
      ctr_q <= ctr_d;
      pwm_q <= pwm_d;
    end
  end
 
endmodule
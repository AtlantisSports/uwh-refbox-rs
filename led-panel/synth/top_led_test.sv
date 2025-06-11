module top_led_test(
    input clk,
    input rst_n,
    output bit [7:0] led,
    output bit ls_10a, ls_10b, ls_10c, ls_10d, ls_10e, ls_10f, ls_10g,
    output bit ls_1a, ls_1b, ls_1c, ls_1d, ls_1e, ls_1f, ls_1g,
    output bit rs_10a, rs_10b, rs_10c, rs_10d, rs_10e, rs_10f, rs_10g,
    output bit rs_1a, rs_1b, rs_1c, rs_1d, rs_1e, rs_1f, rs_1g,
    output bit white_on_left, white_on_right,
    output bit m_10a, m_10b, m_10c, m_10d, m_10e, m_10f, m_10g,
    output bit m_1a, m_1b, m_1c, m_1d, m_1e, m_1f, m_1g,
    output bit s_10a, s_10b, s_10c, s_10d, s_10e, s_10f, s_10g,
    output bit s_1a, s_1b, s_1c, s_1d, s_1e, s_1f, s_1g,
    output bit left_to_ind, right_to_ind, ref_to_ind,
    output bit one, slash, two, overtime, sdn_dth,
    output bit colon,
    input usb_rx,
    output bit usb_tx
    );
    
    wire rst;
    
    reset_conditioner reset_conditioner(.clk(clk), .in(!rst_n), .out(rst));

    reg [10:0] pwm_counter = 11'b0;
    wire pwm_clk;

    assign pwm_clk = pwm_counter[10];

    always @(posedge clk or posedge rst) begin
        if (rst) begin
            pwm_counter <= 11'b0;
        end else begin
            pwm_counter <= pwm_counter + 1;
        end
    end

    wire [74:0] pwm_out;

    // Spacing 3
    reg [7:0] compare [67:0] = {8'hC9, 8'hC6, 8'hC3, 8'hC0, 8'hBD, 8'hBA, 8'hB7, 8'hB4, 8'hB1, 8'hAE, 8'hAB, 8'hA8, 8'hA5, 8'hA2, 8'h9F, 8'h9C, 8'h99, 8'h96, 8'h93, 8'h90, 8'h8D, 8'h8A, 8'h87, 8'h84, 8'h81, 8'h7E, 8'h7B, 8'h78, 8'h75, 8'h72, 8'h6F, 8'h6C, 8'h69, 8'h66, 8'h63, 8'h60, 8'h5D, 8'h5A, 8'h57, 8'h54, 8'h51, 8'h4E, 8'h4B, 8'h48, 8'h45, 8'h42, 8'h3F, 8'h3C, 8'h39, 8'h36, 8'h33, 8'h30, 8'h2D, 8'h2A, 8'h27, 8'h24, 8'h21, 8'h1E, 8'h1B, 8'h18, 8'h15, 8'h12, 8'h0F, 8'h0C, 8'h09, 8'h06, 8'h03, 8'h00};
    wire [66:0] pwm_out;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(67)) pwm(.clk(pwm_clk), .rst(rst), .compare(compare), .pwm(pwm_out));

    assign ls_10a = pwm_out[0];
    assign ls_10b = pwm_out[1];
    assign ls_10c = pwm_out[2];
    assign ls_10d = pwm_out[3];
    assign ls_10e = pwm_out[4];
    assign ls_10f = pwm_out[5];
    assign ls_10g = pwm_out[6];

    assign ls_1a = pwm_out[7];
    assign ls_1b = pwm_out[8];
    assign ls_1c = pwm_out[9];
    assign ls_1d = pwm_out[10];
    assign ls_1e = pwm_out[11];
    assign ls_1f = pwm_out[12];
    assign ls_1g = pwm_out[13];

    assign rs_10a = pwm_out[14];
    assign rs_10b = pwm_out[15];
    assign rs_10c = pwm_out[16];
    assign rs_10d = pwm_out[17];
    assign rs_10e = pwm_out[18];
    assign rs_10f = pwm_out[19];
    assign rs_10g = pwm_out[20];

    assign rs_1a = pwm_out[21];
    assign rs_1b = pwm_out[22];
    assign rs_1c = pwm_out[23];
    assign rs_1d = pwm_out[24];
    assign rs_1e = pwm_out[25];
    assign rs_1f = pwm_out[26];
    assign rs_1g = pwm_out[27];

    assign white_on_left = pwm_out[28];
    assign white_on_right = pwm_out[29];

    assign m_10a = pwm_out[30];
    assign m_10b = pwm_out[31];
    assign m_10c = pwm_out[32];
    assign m_10d = pwm_out[33];
    assign m_10e = pwm_out[34];
    assign m_10f = pwm_out[35];
    assign m_10g = pwm_out[36];

    assign m_1a = pwm_out[37];
    assign m_1b = pwm_out[38];
    assign m_1c = pwm_out[39];
    assign m_1d = pwm_out[40];
    assign m_1e = pwm_out[41];
    assign m_1f = pwm_out[42];
    assign m_1g = pwm_out[43];

    assign s_10a = pwm_out[44];
    assign s_10b = pwm_out[45];
    assign s_10c = pwm_out[46];
    assign s_10d = pwm_out[47];
    assign s_10e = pwm_out[48];
    assign s_10f = pwm_out[49];
    assign s_10g = pwm_out[50];

    assign s_1a = pwm_out[51];
    assign s_1b = pwm_out[52];
    assign s_1c = pwm_out[53];
    assign s_1d = pwm_out[54];
    assign s_1e = pwm_out[55];
    assign s_1f = pwm_out[56];
    assign s_1g = pwm_out[57];

    assign left_to_ind = pwm_out[58];
    assign right_to_ind = pwm_out[59];
    assign ref_to_ind = pwm_out[60];

    assign one = pwm_out[61];
    assign slash = pwm_out[62];
    assign two = pwm_out[63];
    assign overtime = pwm_out[64];
    assign sdn_dth = pwm_out[65];

    assign colon = pwm_out[66];

    assign led[7:0] = pwm_out[7:0];

endmodule
